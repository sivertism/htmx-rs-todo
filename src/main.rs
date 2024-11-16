mod grocy;
mod template;
mod todo;
mod database;

#[allow(unused_imports)]
use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, get_service, post},
    Form,
};
use grocy::*;
use database::Database;
use serde::Deserialize;
use anyhow::Context;
use template::*;
#[allow(unused_imports)]
use todo::{List, ListForm, Task, TaskForm};
use tokio::net::TcpListener;
use tower_http::services::ServeFile;
use tracing_subscriber;
#[allow(unused_imports)]
use tracing::{info, warn, debug};
use clap::Parser;

/// Crappy todo app to test out HTMX with Rust as the backend
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Data storage directory
    #[arg(long, default_value = ".")]
    data_dir: std::path::PathBuf,

    /// Listening port
    #[arg(short, long, default_value_t = 3000)]
    port: u16,

    /// Listening address
    #[arg(short, long, default_value = "127.0.0.1")]
    address: String,
}


#[derive(Clone)]
struct AppState {
    db: Database,
}

#[derive(Deserialize)]
struct ListQuery {
    list_id: Option<usize>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let cli = Cli::parse();

    tracing_subscriber::fmt::init();

    let db = Database::new(cli.data_dir.join("todos.db")).await.context("Create db")?;

    let state = AppState {
        db
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
    let listen_address = format!("{}:{}", cli.address, cli.port);
    let listener = TcpListener::bind(listen_address).await?;
    info!("listening on {}", listener.local_addr().unwrap());

    // Start the Axum server with the defined routes
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index(list_query: Query<ListQuery>) -> impl IntoResponse {
    let selected_list = match list_query.list_id {
        Some(id) => id,
        None => 3,
    };

    HtmlTemplate(IndexTemplate {selected_list})
}

async fn get_tasks(State(state): State<AppState>, Path(list_id): Path<u32>) -> impl IntoResponse {
    // 1. Get Grocy credentials for the list if they exist
    if let Some(gc) = state.db.get_grocy_credentials(list_id as usize).await {
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
                    
                    // 3. Create task from grocy
                    if let Ok(_id) = state.db.create_task(task, list_id as usize).await {
                        // 4. Delete from Grocy
                        delete_shopping_list_item(item.id, &gc).await.expect("Failed to delete");
                    } else {
                      warn!("Failed to create task from Grocy shopping list item"); 
                    }
                }
        },
            Err(err) => {warn!("Failed to get shopping list items: {}", err);}
        }
    }

    if let Ok(tasks) = state.db.get_tasks(list_id as usize).await {
        let incomplete = tasks.into_iter().filter(|task| !task.completed).collect();
        info!(
            "Got incomplete tasks: {:?} from list with id {:?}",
            incomplete, list_id
        );
        let template = TasksTemplate { tasks: incomplete };
        HtmlTemplate(template).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn get_list_options(State(state): State<AppState>, list_query: Query<ListQuery>) -> impl IntoResponse {
    let selected_list = match list_query.list_id {
        Some(id) => id,
        None => 1,
    };

    let lists = state.db.get_lists().await.expect("Get list options");

    info!("Got {} lists", lists.len());
    let template = ListOptionsTemplate { lists, selected_list};
    HtmlTemplate(template)
}

async fn get_completed(
    State(state): State<AppState>,
    Path(list_id): Path<u32>,
) -> impl IntoResponse {
    if let Ok(tasks) = state.db.get_tasks(list_id as usize).await {
        let completed = tasks.into_iter().filter(|task| task.completed).collect();
        info!(
            "Got completed tasks: {:?} from list with id {:?}",
            completed, list_id
        );
        let template = TasksTemplate { tasks: completed };
        HtmlTemplate(template).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

// delete task handler
async fn delete_task(State(state): State<AppState>, Path(id): Path<u32>) -> StatusCode {
    state.db.delete_task(id as usize).await.expect("Delete task");
    info!("Deleted task with id {}", id);
    StatusCode::OK
}

// complete tasks handler
async fn toggle_task(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    let id = id as usize;
    info!("Toggling task with id {}", id);
    if let Ok(_) = state.db.toggle_task_completed(id).await {
        if let Ok(task) = state.db.get_task(id).await {
            return HtmlTemplate(TasksCompleteTemplate { task }).into_response();
        } else {
            warn!("Toggled task with id {}, but failed to retrieve it!", id);
            StatusCode::OK.into_response()
        }
    } else {
        warn!("Failed to toggle task with id {}", id);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
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
    let list = state.db.get_list(selected_list).await.expect("Get a list");
    HtmlTemplate(ListTablesTemplate { list })
}

async fn create_task(
    State(state): State<AppState>,
    Path(list_id): Path<u32>,
    form: Form<TaskForm>,
) -> impl IntoResponse {
    let text = form.text.clone();
    info!("Inserting task item with list_id {}", list_id);

    let id = state.db.create_task(text, list_id as usize).await.expect("Create task on db");

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

    let grocy_credentials = match (form.grocy_url.clone(), form.grocy_api_key.clone()) {
        (Some(url), Some(api_key)) => Some(GrocyCredentials{url, api_key}),
        (Some(_url), None) => {
            warn!("Grocy URL provided, but no API key.");
            None
            },
        (None, Some(_api_key)) => {
            warn!("Grocy API key provided, but no URL");
            None
            },
        (None, None) => None
    };

    if let Ok(id) = state.db.create_list(name, grocy_credentials.as_ref()).await.context("Create list") {
        info!("List item with id {} created", id);
        StatusCode::CREATED
    } else {
        StatusCode::NOT_ACCEPTABLE
    }
}
