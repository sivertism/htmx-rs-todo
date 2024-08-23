mod template;
mod todo;
mod database;

#[allow(unused_imports)]
use axum::{
    Form,
    extract::Path,
    response::{IntoResponse, Redirect},
    routing::{delete, get, post},
    http::{StatusCode, Uri},
    BoxError,
    Router
};
use axum_server::tls_rustls::RustlsConfig;
use std::{net::SocketAddr, path::PathBuf};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use todo::{Todo,TodoForm};
use template::{HtmlTemplate, IndexTemplate, TodosTemplate, TodosCompleteTemplate};

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct Ports {
    http: u16,
    https: u16,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    
    tracing_subscriber::registry()
    .with(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "example_tls_rustls=debug".into()),
    )
    .with(tracing_subscriber::fmt::layer())
    .init();

    let ports = Ports {
        http: 7878,
        https: 3000,
    };

    // configure certificate and private key used by https
    // #TODO use options to set path to certs
    let config = RustlsConfig::from_pem_file(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("self_signed_certs")
            .join("cert.pem"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("self_signed_certs")
            .join("key.pem"),
    )
    .await
    .unwrap();

    let app = axum::Router::new() // Create a new Axum Router
            .route("/", get(index)) // Define a GET route for the root path, handled by the
                                     // `index` function
            .route("/todo", get(get_todos).post(create_todo))
            .route("/completed", get(get_completed).post(toggle_todo))
            // :id defines path parameters for our route
            .route("/todo/:id", delete(delete_todo).post(toggle_todo));

    // Run https server
    let addr = SocketAddr::from(([0,0,0,0], ports.https));
    tracing::debug!("Listening on {}", addr);
    println!("Listening on {}", addr);
    axum_server::bind_rustls(addr, config)
      .serve(app.into_make_service())
      .await?;
    Ok(())
}

async fn index() -> impl IntoResponse {
    // return HtlmTemplate to make it a valid repsonse
    HtmlTemplate(IndexTemplate {})
}

// get todos handler
async fn get_todos() -> impl IntoResponse {
    let todos =  database::get_todos(false).expect("failed to get todos");
    println!("Got todos: {:?}", todos);
    let template = TodosTemplate {todos: todos};
    HtmlTemplate(template)
}

async fn get_completed() -> impl IntoResponse {
    let todos =  database::get_todos(true).expect("failed to get todos");
    println!("Got todos: {:?}", todos);
    let template = TodosTemplate {todos: todos};
    HtmlTemplate(template)
}

// delete todo handler
async fn delete_todo(Path(id): Path<u32>) -> StatusCode {
    database::delete_todo(id as usize);
    //HtmlTemplate(TodosTemplate { todos })
    StatusCode::OK
}

// complete todos handler
async fn toggle_todo(Path(id): Path<u32>) -> impl IntoResponse {
    let template = TodosCompleteTemplate { todo: database::toggle_todo(id as usize).expect("failed to complete todo item")};
    HtmlTemplate(template)
}

pub async fn create_todo(
    form: Form<TodoForm>
    ) -> impl IntoResponse {
    let id = database::create_todo(form.text.clone());
    let todos = vec![Todo{id: id, text: form.text.clone(), completed: false}];
    println!("Todo item with id {} created", id);

    // could just return one todo if we fix the template to only add an item!
    HtmlTemplate(TodosTemplate {todos})
}
