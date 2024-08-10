mod template;
mod todo;
mod database;

use axum::extract::Path;
use axum::Form;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use tokio::net::TcpListener;
use todo::TodoForm;
use template::{HtmlTemplate, IndexTemplate, TodosTemplate};

#[tokio::main]
async fn main() -> std::io::Result<()> {

    let app = axum::Router::new() // Create a new Axum Router
            .route("/", get(index)) // Define a GET route for the root path, handled by the
                                     // `index` function
            .route("/todo", get(get_todos).post(create_todo))
            // :id defines path parameters for our route
            .route("/todo/:id", delete(delete_todo));

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

// get todos handler
async fn get_todos() -> impl IntoResponse {
    let template = TodosTemplate { todos: database::get_todos().expect("failed to get todos")};
    HtmlTemplate(template)
}

// delete todo handler
async fn delete_todo(Path(id): Path<u32>) -> impl IntoResponse {
    database::delete_todo(id as usize);
    let todos = database::get_todos().expect("failed to get todos");
    HtmlTemplate(TodosTemplate { todos })
}

pub async fn create_todo(
    form: Form<TodoForm>
    ) -> impl IntoResponse {
    database::create_todo(form.text.clone());
    let todos = database::get_todos().expect("Failed to get todos");
    // could just return one todo if we fix the template to only add an item!
    HtmlTemplate(TodosTemplate { todos })
}
