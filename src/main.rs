mod grocy;
mod template;
mod todo;

#[allow(unused_imports)]
use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, get_service, post},
    Form,
};
use grocy::*;
use rusqlite;
use serde::Deserialize;
use anyhow::Context;
use template::*;
use todo::{List, ListForm, Task, TaskForm};
use tokio::net::TcpListener;
#[allow(unused_imports)]
use tokio_rusqlite::{params, Connection};
use tower_http::services::ServeFile;
use tracing_subscriber;
use tracing::{info, warn, debug};

#[derive(Debug, Clone)]
struct AppState {
    dbconn: Connection,
}

#[derive(Deserialize)]
struct ListQuery {
    list_id: Option<usize>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    // Create, or connect to a local SQLite database to store the tasks
    let dbconn = Connection::open("todos.db")
        .await
        .context("Open database")?;

    // Insert basic tracing function to print sql queries to console
    dbconn
        .call(|conn| {
            conn.trace(Some(|statement| {
                debug!("{}", statement); })); 
                Ok(()) 
        })
        .await
        .context("Add tracing function")?;

    // Initialize db
    initialize_database(&dbconn)
        .await
        .context("Initialize database")?;

    let state = AppState {
        dbconn,
    };

    let app = axum::Router::new() // Create a new Axum Router
        .route("/", get(index)) // Define a GET route for the root path, handled by the
        // `index` function
        .route("/list_tables", get(get_list_tables))
        .route("/lists", get(get_list_options))
        .route("/task/:id", delete(delete_task).post(toggle_task))
        .route("/:list_id/task", get(get_tasks).post(create_task))
        .route("/:list_id/completed", get(get_completed).post(toggle_task))
        // :id defines path parameters for our route
        .route(
            "/create_list",
            get_service(ServeFile::new("templates/create_list.html")).post(create_list),
        )
        .with_state(state);

    // Bind a TCP listener to the specified address
    let listener = TcpListener::bind("127.0.0.1:3000").await?;
    info!("listening on {}", listener.local_addr().unwrap());

    // Start the Axum server with the defined routes
    axum::serve(listener, app).await?;

    Ok(())
}

async fn initialize_database(dbconn: &Connection) -> anyhow::Result<()> {
    dbconn
        .call(|conn| {
            let sql_schema = include_str!("../sql/schema.sql");
            conn.execute_batch(sql_schema)
                .expect("Failed to execute database schema");
            Ok(())
        })
        .await?;
    Ok(())
}

async fn index(list_query: Query<ListQuery>) -> impl IntoResponse {
    let selected_list = match list_query.list_id {
        Some(id) => id,
        None => 3,
    };

    HtmlTemplate(IndexTemplate {selected_list})
}

async fn get_grocy_credentials(
    list_id: usize,
    dbconn: &tokio_rusqlite::Connection,
) -> Option<GrocyCredentials> {
    if let Ok(res) = dbconn
        .call(move |conn| {
            let res = conn.query_row(
                "SELECT url, api_key FROM grocy_credentials WHERE list_id=(:list_id)",
                &[(":list_id", &list_id)],
                |row| {
                    Ok(GrocyCredentials {
                        url: row.get(0)?,
                        api_key: row.get(1)?,
                    })
                },
            )?;
            Ok(res)
        })
        .await
    {
        return Some(res);
    }
    return None;
}

async fn get_tasks(State(state): State<AppState>, Path(list_id): Path<u32>) -> impl IntoResponse {
    // 1. Get Grocy credentials for the list if they exist
    if let Some(gc) = get_grocy_credentials(list_id as usize, &state.dbconn).await {
        info!("Got credentials for {}", gc.url);

        // 2. Get Grocy shopping list items
        match grocy::get_shopping_list_items(&gc).await {
            Ok(items) => {
                info!("Got {} items from Grocy", items.len());

                for item in items.into_iter() {
                    let name = get_product_name(item.product_id, &gc).await.expect("Failed to get product name");
                    let quantity_unit = get_quantity_unit(item.quantity_unit_id, &gc).await.expect("Failed to get quantity unit");
                    let task = format!("{} {} {}", name, item.amount, quantity_unit);
                    info!("Consuming task '{}' from Grocy", &task);
                    let _id = state
                        .dbconn
                        .call(move |conn| {
                            match conn.execute(
                                "INSERT INTO tasks (task, list_id) values (?1, ?2)",
                                rusqlite::params![&task, &list_id],
                            ) {
                                Ok(_n_updated) => {}
                                Err(err) => {
                                    warn!("Create task from grocy failed: {}", err);
                                }
                            }
                            Ok(conn.last_insert_rowid() as usize)
                        })
                        .await
                        .expect("Failed to create task on db.");

                    // Delete from Grocy
                    delete_shopping_list_item(item.id, &gc).await.expect("Failed to delete");
                }
        },
            Err(err) => {warn!("Failed to get shopping list items: {}", err);}
        }
    }

    let tasks = state
        .dbconn
        .call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT tasks.id, tasks.task, tasks.completed 
            FROM tasks 
            INNER JOIN lists ON lists.id=tasks.list_id 
            WHERE completed=0 AND lists.id=(:list_id) 
            ORDER BY tasks.modified DESC;",
            )?;
            let rows = stmt.query_map(&[(":list_id", &list_id)], |row| {
                Ok(Task {
                    id: row.get(0)?,
                    text: row.get(1)?,
                    completed: row.get(2)?,
                    list_id: list_id as usize,
                })
            })?;
            let mut tasks = Vec::new();
            for r in rows {
                tasks.push(r?);
            }
            Ok(tasks)
        })
        .await
        .expect("Failed to get tasks");

    info!("Got tasks: {:?} from list with id {:?}", tasks, list_id);
    let template = TasksTemplate { tasks };
    HtmlTemplate(template)
}

async fn get_list_options(State(state): State<AppState>, list_query: Query<ListQuery>) -> impl IntoResponse {
    let selected_list = match list_query.list_id {
        Some(id) => id,
        None => 1,
    };
    let lists = state
        .dbconn
        .call(move |conn| {
            let mut stmt = conn.prepare("SELECT id, name FROM lists")?;
            let rows = stmt.query_map([], |row| {
                Ok(List {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })?;
            let mut lists = Vec::new();
            for r in rows {
                lists.push(r?);
            }
            Ok(lists)
        })
        .await
        .expect("Failed to get lists");

    info!("Got lists: {:?}", lists);
    let template = ListOptionsTemplate { lists, selected_list};
    HtmlTemplate(template)
}

async fn get_completed(
    State(state): State<AppState>,
    Path(list_id): Path<u32>,
) -> impl IntoResponse {
    let tasks = state
        .dbconn
        .call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT tasks.id, tasks.task, tasks.completed 
            FROM tasks 
            INNER JOIN lists ON lists.id=tasks.list_id 
            WHERE completed=1 AND lists.id=(:list_id) 
            ORDER BY tasks.modified DESC;",
            )?;
            let rows = stmt.query_map(&[(":list_id", &list_id)], |row| {
                Ok(Task {
                    id: row.get(0)?,
                    text: row.get(1)?,
                    completed: row.get(2)?,
                    list_id: list_id as usize,
                })
            })?;
            let mut tasks = Vec::new();
            for r in rows {
                tasks.push(r?);
            }
            Ok(tasks)
        })
        .await
        .expect("Failed to get tasks");

    info!(
        "Got completed tasks: {:?} from list with id {:?}",
        tasks, list_id
    );
    let template = TasksTemplate { tasks };
    HtmlTemplate(template)
}

// delete task handler
async fn delete_task(State(state): State<AppState>, Path(id): Path<u32>) -> StatusCode {
    state
        .dbconn
        .call(
            move |conn| match conn.execute("DELETE FROM tasks WHERE id=(?1)", &[&id]) {
                Ok(_n_updated) => {
                    Ok(())
                }
                Err(err) => {
                    println!("Delete failed: {}", err);
                    Ok(())
                }
            },
        )
        .await
        .expect("Failed to delete task");
    info!("Deleted task with id {}",  id);
    StatusCode::OK
}

// complete tasks handler
async fn toggle_task(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    info!("Toggling task with id {}", id);
    let task = state
        .dbconn
        .call(move |conn| {
            match conn.execute(
                "UPDATE tasks 
                           SET completed = ((completed | 1) - (completed & 1)) 
                           WHERE id=(?1)",
                &[&id],
            ) {
                Ok(updated) => {
                    info!("{} rows were updated", updated);
                }
                Err(err) => {
                    panic!("Failed to update row with {}", err);
                }
            };
            Ok(
                conn.query_row("SELECT * FROM tasks WHERE id=(?1)", &[&id], |row| {
                    Ok(Task {
                        id: row.get(0)?,
                        text: row.get(1)?,
                        completed: row.get(2)?,
                        list_id: id as usize,
                    })
                }),
            )
        })
        .await
        .expect("Failed to update task");
    match task {
        Ok(task) => HtmlTemplate(TasksCompleteTemplate { task }),
        Err(err) => {
            panic!("Failed to update task with {}", err);
        }
    }
}

// complete tasks handler
async fn get_list_tables(
    State(state): State<AppState>,
    list_query: Query<ListQuery>,
) -> impl IntoResponse {
    let selected_list = match list_query.list_id {
        Some(id) => id,
        None => 1,
    };
    let list = state
        .dbconn
        .call(move |conn| {
            Ok(conn.query_row(
                "SELECT * FROM lists WHERE id=(?1)",
                &[&selected_list],
                |row| {
                    Ok(List {
                        id: row.get(0)?,
                        name: row.get(1)?,
                    })
                },
            ))
        })
        .await
        .expect("Failed to retrieve list")
        .unwrap();
    HtmlTemplate(ListTablesTemplate { list })
}

async fn create_task(
    State(state): State<AppState>,
    Path(list_id): Path<u32>,
    form: Form<TaskForm>,
) -> impl IntoResponse {
    let text = form.text.clone();
    info!("Inserting task item with list_id {}", list_id);

    let id = state
        .dbconn
        .call(move |conn| {
            match conn.execute(
                "INSERT INTO tasks (task, list_id) values (?1, ?2)",
                rusqlite::params![&text, &list_id],
            ) {
                Ok(updated) => {
                    info!("{} rows were inserted", updated);
                }
                Err(err) => {
                    panic!("Create task failed: {}", err);
                }
            }
            Ok(conn.last_insert_rowid() as usize)
        })
        .await
        .expect("Failed to create task on db.");
    info!("Task item with id {} created", id);

    let tasks = vec![Task {
        id: id,
        text: form.text.clone(),
        completed: false,
        list_id: list_id as usize,
    }];

    // could just return one task if we fix the template to only add an item!
    HtmlTemplate(TasksTemplate { tasks })
}

async fn create_list(State(state): State<AppState>, form: Form<ListForm>) -> StatusCode {
    let name = form.name.clone();

    let id = state
        .dbconn
        .call(move |conn| {
            // Create the list
            match conn.execute("INSERT INTO lists (name) values (?1)", &[&name]) {
                Ok(_n_updated) => {}
                Err(err) => {
                    warn!("Create list failed: {}", err);
                }
            }
            Ok(conn.last_insert_rowid() as usize)
        })
        .await
        .expect("Failed to create list on db.");
    // Store grocy info, connected to list
    if let Some(grocy_url) = form.grocy_url.clone() {
        if let Some(grocy_api_key) = form.grocy_api_key.clone() {
            println!("Inserting Grocy credentials for {}", grocy_api_key);
            state
                .dbconn
                .call(move |conn| {
                    // Create the list
                    match conn.execute(
                        "INSERT INTO grocy_credentials (url, api_key, list_id) values (?1, ?2, ?3)",
                        rusqlite::params![&grocy_url, &grocy_api_key, &id],
                    ) {
                        Ok(updated) => {
                            println!("{} rows were inserted", updated);
                        }
                        Err(err) => {
                            panic!("Failed to store Grocy credentials: {}", err);
                        }
                    }
                    Ok(())
                })
                .await
                .expect("Failed to store Grocy credentials.");
        } else {
            println!("Grocy URL supplied, but no API key. Ignoring.");
        }
    }

    println!("List item with id {} created", id);
    StatusCode::OK
}
