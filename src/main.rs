mod template;
mod todo;

use axum::extract::{Path, Form};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use tokio::net::TcpListener;
use template::{HtmlTemplate, IndexTemplate};

// Path to the JSON file for storing todos
const TODO_FILE_PATH: &str = "./src/todos.json";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    println!("Hello, world!");
    let app = axum::Router::new() // Create a new Axum Router
            .route("/", get(index)); // Define a GET route for the root path, handled by the
                                     // `index` function

    // Bind a TCP listener to the specified address
    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    println!("listening on {}", listener.local_addr().unwrap());

    // Start the Axum server with the defined routes
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> impl IntoResponse {
    // return HtlmTemplate to make it a valid repsonse
    HtmlTemplate(IndexTemplate {})
}
