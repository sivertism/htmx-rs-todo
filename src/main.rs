mod grocy;
mod template;
mod todo;
mod database;

#[allow(unused_imports)]
use axum::{
    extract::{Extension, Path, Query, State, Json},
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Response, Html, Redirect},
    routing::{delete, get, get_service, post},
    Form,
};
use grocy::*;
use database::Database;
use reqwest::header;
use serde::Deserialize;
use anyhow::Context;
use template::*;
#[allow(unused_imports)]
use todo::{List, ListForm, Task, TaskForm};
use tokio::net::TcpListener;
use tower_http::services::{ServeFile, ServeDir};
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

#[derive(Deserialize)]
struct ReorderPayload {
    order: Vec<u64>,
}

const HTMX_JS_GZIP: &[u8] = include_bytes!("../vendor/htmx.js.gz");
const SORTABLE_JS_GZIP: &[u8] = include_bytes!("../vendor/Sortable.js.gz");
const PICO_CSS_GZIP: &[u8] = include_bytes!("../vendor/pico.css.gz");

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let cli = Cli::parse();

    tracing_subscriber::fmt::init();

    let dbpath = cli.data_dir.join("todos.db");

    info!("Opening database at {:?}", dbpath);

    let db = Database::new(dbpath).await.context("Create db")?;

    let state = AppState {
        db
    };

    let app = axum::Router::new()
        .route("/", get(index))
        .route("/manage", get(manage).post(create_list))
        .route("/list/:id", delete(delete_list))
        .route("/task/:id", delete(delete_task).post(toggle_task))
        .route("/:list_id/task", post(create_task))
        .route( "/create_list", post(create_list))
        .route("/reorder", post(reorder))
        .route("/vendor/htmx.js", get(htmx))
        .route("/vendor/Sortable.js", get(sortable))
        .route("/vendor/pico.min.css", get(picocss))
        .with_state(state);

    // Bind a TCP listener to the specified address
    let listen_address = format!("{}:{}", cli.address, cli.port);
    let listener = TcpListener::bind(listen_address).await?;
    info!("listening on {}", listener.local_addr().unwrap());

    // Start the Axum server with the defined routes
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index(list_query: Query<ListQuery>, State(state): State<AppState>) -> impl IntoResponse {
    let selected_list = match list_query.list_id {
        Some(id) => id,
        None => 3,
    };

    let lists = state.db.get_lists().await.expect("Get list options");

    // 1. Get Grocy credentials for the list if they exist
    if let Some(gc) = state.db.get_grocy_credentials(selected_list as usize).await {
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
                    if let Ok(_id) = state.db.create_task(task, selected_list as usize).await {
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

    info!("Getting tasks for list_id={}", selected_list);
    if let Ok(tasks) = state.db.get_tasks(selected_list as usize).await {
        let incomplete : Vec<Task>= tasks.clone().into_iter().filter(|task| !task.completed).collect();
        info!(
            "Got incomplete tasks: {:?} from list with id {:?}",
            incomplete, selected_list
        );
        let template = IndexTemplate { selected_list, lists, tasks};
        HtmlTemplate(template).into_response()
    } else {
        warn!("Failed to get tasks for list_id={}", selected_list);
        let tasks: Vec<Task> = vec![];
        let template = IndexTemplate { selected_list, lists, tasks};
        HtmlTemplate(template).into_response()
    }
}

async fn delete_task(State(state): State<AppState>, Path(id): Path<u32>) -> StatusCode {
    state.db.delete_task(id as usize).await.expect("Delete task");
    info!("Deleted task with id {}", id);
    StatusCode::OK
}

async fn delete_list(State(state): State<AppState>, Path(id): Path<u32>) -> impl IntoResponse {
    state.db.delete_list(id as usize).await.expect("Delete list");
    info!("Deleted list with id {}", id);

    // Need to use HX-Redirect to force redirect when using HTMX
    let mut headers = HeaderMap::new();
    headers.insert("HX-Redirect", "/manage".parse().unwrap());
    (headers, "").into_response()
}

async fn toggle_task(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    let id = id as usize;
    info!("Toggling task with id {}", id);
    if let Ok(_) = state.db.toggle_task_completed(id).await {
        if let Ok(task) = state.db.get_task(id).await {
            return HtmlTemplate(TaskTemplate { task }).into_response();
        } else {
            warn!("Toggled task with id {}, but failed to retrieve it!", id);
            StatusCode::OK.into_response()
        }
    } else {
        warn!("Failed to toggle task with id {}", id);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
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

    let task = Task {
        id: id,
        text: form.text.clone(),
        completed: false,
        list_id: list_id as usize,
    };

    // could just return one task if we fix the template to only add an item!
    HtmlTemplate(TaskTemplate { task })
}

async fn create_list(State(state): State<AppState>, form: Form<ListForm>) -> Response {
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

    if let Ok(id) = state.db.create_list(name.clone(), grocy_credentials.as_ref()).await.context("Create list") {
        info!("List item with id {} created", id);
        return Html(format!(r#"<option class="select-list" value="?list_id={id}">{name}</option>"#)).into_response();
    } else {
        warn!("Failed to create list");
        return StatusCode::BAD_REQUEST.into_response();
    }

}

async fn manage(list_query: Query<ListQuery>, State(state): State<AppState>) -> impl IntoResponse {
    let selected_list = match list_query.list_id {
        Some(id) => id,
        None => 3,
    };

    let lists = state.db.get_lists().await.expect("Get list options");
    let template = ManageTemplate { selected_list, lists};
    HtmlTemplate(template).into_response()
}

async fn htmx() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/javascript".parse().unwrap());
    headers.insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
    (headers, HTMX_JS_GZIP)
}

async fn sortable() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/javascript".parse().unwrap());
    headers.insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
    (headers, SORTABLE_JS_GZIP)
}

async fn picocss() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "text/css".parse().unwrap());
    headers.insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
    (headers, PICO_CSS_GZIP)
}

async fn reorder(State(state): State<AppState>,
                 Query(params): Query<ListQuery>, 
                 Json(payload): Json<ReorderPayload>,
                 ) -> StatusCode {
    println!("List {:?} reordered to {:?}", params.list_id, payload.order);
    //state.db.reorder(params.list_id.unwrap(), payload.order).await.expect("reorder");
    //StatusCode::OK
    match state.db.reorder(params.list_id.unwrap(), payload.order).await {
        Ok(()) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
