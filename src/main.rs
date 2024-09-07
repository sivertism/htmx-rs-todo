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
use reqwest;
use rusqlite;
use serde::Deserialize;
use template::*;
use todo::{List, ListForm, Task, TaskForm};
use tokio::net::TcpListener;
#[allow(unused_imports)]
use tokio_rusqlite::{params, Connection, Result};
use tower_http::services::ServeFile;

#[derive(Debug, Clone)]
struct AppState {
    dbconn: Connection,
}

#[derive(Deserialize)]
struct ListQuery {
    list_id: usize,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Create, or connect to a local SQLite database to store the tasks
    let dbconn = Connection::open("todos.db")
        .await
        .expect("Failed to open database");

    // Insert basic tracing function to print sql queries to console
    dbconn
        .call(|conn| {
            conn.trace(Some(|statement| {
                println!("{}", statement);
            }));
            Ok(())
        })
        .await
        .expect("Failed to add tracing function");

    // Initialize db
    initialize_database(&dbconn)
        .await
        .expect("Failed to initialize database");

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
    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    println!("listening on {}", listener.local_addr().unwrap());

    // Start the Axum server with the defined routes
    axum::serve(listener, app).await?;

    Ok(())
}

async fn initialize_database(dbconn: &Connection) -> Result<()> {
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

async fn index() -> impl IntoResponse {
    HtmlTemplate(IndexTemplate {})
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

async fn insert_grocy_shopping_list_items(
    list_id: usize,
    grocy_shopping_list_items: Vec<ShoppingListItem>,
    dbconn: &tokio_rusqlite::Connection,
) {
    // Create tasks from items
    // filter out tasks that are already in the grocy-to-task map
    // Insert if not
    // Get list of items that are not already inserted
    let mut new_items = vec![];
    for item in grocy_shopping_list_items.into_iter() {
        let exists: bool = dbconn.call(move | conn | {
                let mut stmt = conn.prepare("SELECT grocy_id FROM grocy_tasks_mapping WHERE grocy_id=(?1) AND list_id=(?2)")?;
                Ok(stmt.exists(rusqlite::params![&item.id, &list_id])?)
            })
            .await
            .expect("Failed to get new ids");

        if !exists {
            new_items.push(item);
        }

    }

    println!("Got {} new items from Grocy", new_items.len());

    // NOTE: Should be done in a transaction, so prepare new Task
    // then run single execute to insert into both tables.
    // Insert new items into task mapping
    
    // Insert new items into tasks
}

// get tasks handler
async fn get_tasks(State(state): State<AppState>, Path(list_id): Path<u32>) -> impl IntoResponse {
    // 1. Get Grocy credentials for the list if they exist
    if let Some(gc) = get_grocy_credentials(list_id as usize, &state.dbconn).await {
        println!("Got credentials for {}", gc.url);

        // 2. Get Grocy shopping list items
        match grocy::get_shopping_list_items(&gc).await {
            Ok(items) => {
                println!("Got {} items from Grocy", items.len());

                // Insert into database
                //insert_grocy_shopping_list_items(list_id as usize, items, &state.dbconn).await;

                for item in items.into_iter() {
                    println!("Getting product name of {:?}", item);
                    let name = get_product_name(item.product_id, &gc).await.expect("Failed to get product name");
                    let quantity_unit = get_quantity_unit(item.quantity_unit_id, &gc).await.expect("Failed to get quantity unit");
                    let task = format!("{} {} {}", name, item.amount, quantity_unit);
                    println!("Got task {}", task);
                    let id = state
                        .dbconn
                        .call(move |conn| {
                            match conn.execute(
                                "INSERT INTO tasks (task, list_id) values (?1, ?2)",
                                rusqlite::params![&task, &list_id],
                            ) {
                                Ok(updated) => {
                                    println!("{} rows were inserted", updated);
                                }
                                Err(err) => {
                                    panic!("Create task failed: {}", err);
                                }
                            }
                            Ok(conn.last_insert_rowid() as usize)
                        })
                        .await
                        .expect("Failed to create task on db.");

                    // Delete from Grocy
                    delete_shopping_list_item(item.id, &gc).await.expect("Failed to delete");
                }
                //println!("Task item with id {} created", id);

                // Delete from Grocy
        },
            Err(err) => {println!("Failed to get shopping list items. {}", err);}
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

    println!("Got tasks: {:?} from list with id {:?}", tasks, list_id);
    let template = TasksTemplate { tasks };
    HtmlTemplate(template)
}

async fn get_list_options(State(state): State<AppState>) -> impl IntoResponse {
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

    println!("Got lists: {:?}", lists);
    let template = ListOptionsTemplate { lists };
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

    println!(
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
                Ok(updated) => {
                    println!("{} rows were deleted", updated);
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
    StatusCode::OK
}

// complete tasks handler
async fn toggle_task(
    State(state): State<AppState>,
    Path(id): Path<u32>,
    Path(list_id): Path<u32>,
) -> impl IntoResponse {
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
                    println!("{} rows were updated", updated);
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
    let list = state
        .dbconn
        .call(move |conn| {
            Ok(conn.query_row(
                "SELECT * FROM lists WHERE id=(?1)",
                &[&list_query.list_id],
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
    println!("Inserting task item with list_id {}", list_id);

    let id = state
        .dbconn
        .call(move |conn| {
            match conn.execute(
                "INSERT INTO tasks (task, list_id) values (?1, ?2)",
                rusqlite::params![&text, &list_id],
            ) {
                Ok(updated) => {
                    println!("{} rows were inserted", updated);
                }
                Err(err) => {
                    panic!("Create task failed: {}", err);
                }
            }
            Ok(conn.last_insert_rowid() as usize)
        })
        .await
        .expect("Failed to create task on db.");
    println!("Task item with id {} created", id);

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
                Ok(updated) => {
                    println!("{} rows were inserted", updated);
                }
                Err(err) => {
                    panic!("Create list failed: {}", err);
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
