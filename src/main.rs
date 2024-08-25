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
    conn: Connection,
}

#[derive(Deserialize)]
struct ListQuery {
    list_id: usize,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Create, or connect to a local SQLite database to store the tasks
    let conn = Connection::open("todos.db")
        .await
        .expect("Failed to open database");

    // Insert basic tracing function to print sql queries to console
    conn.call(|conn| {
        conn.trace(Some(|statement| {
            println!("{}", statement);
        }));
        Ok(())
    })
    .await
    .expect("Failed to add tracing function");

    // Initialize db
    initialize_database(&conn)
        .await
        .expect("Failed to initialize database");

    let state = AppState { conn };

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

async fn initialize_database(conn: &Connection) -> Result<()> {
    conn.call(|conn| {
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

// get tasks handler
async fn get_tasks(State(state): State<AppState>, Path(list_id): Path<u32>) -> impl IntoResponse {
    let tasks = state
        .conn
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
        .conn
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
        .conn
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
        .conn
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
async fn toggle_task(State(state): State<AppState>, Path(id): Path<u32>) -> impl IntoResponse {
    let task = state
        .conn
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
                        list_id: 1,
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
        .conn
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
        .conn
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
        .conn
        .call(move |conn| {
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
    println!("List item with id {} created", id);
    StatusCode::OK
}
