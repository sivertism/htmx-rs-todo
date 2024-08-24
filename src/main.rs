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
use clap::Parser;

/// Crappy todo app to test out HTMX with Rust as the backend
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {

    /// Public certificate file path
    #[arg(long, default_value_t = String::from("self_signed_certs/cert.pem"))]
    certificate_file_path: String,

    /// Private certificate file key path
    #[arg(long, default_value_t = String::from("self_signed_certs/key.pem"))]
    certificate_key_file_path: String,

    /// HTTP port
    #[arg(long, default_value_t = 7878)]
    http_port: u16,

    /// HTTPS port
    #[arg(long, default_value_t = 3000)]
    https_port: u16,
}


#[tokio::main]
async fn main() -> std::io::Result<()> {

    let cli = Cli::parse();
    
    tracing_subscriber::registry()
    .with(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "example_tls_rustls=debug".into()),
    )
    .with(tracing_subscriber::fmt::layer())
    .init();

    // configure certificate and private key used by https
    let config = RustlsConfig::from_pem_file(
        PathBuf::from(cli.certificate_file_path),
        PathBuf::from(cli.certificate_key_file_path),
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
    let addr = SocketAddr::from(([0,0,0,0], cli.https_port));
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
