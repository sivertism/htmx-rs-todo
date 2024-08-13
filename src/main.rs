mod template;
mod todo;
mod database;

use axum::extract::Path;
use axum::Form;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::http::StatusCode;
use tokio::net::TcpListener;
use todo::{Todo,TodoForm};
use template::{HtmlTemplate, IndexTemplate, TodosTemplate};

#[tokio::main]
async fn main() -> std::io::Result<()> {

    let app = axum::Router::new() // Create a new Axum Router
            .route("/", get(index)) // Define a GET route for the root path, handled by the
                                     // `index` function
            .route("/todo", get(get_todos).post(create_todo))
            .route("/completed", get(get_completed))
            // :id defines path parameters for our route
            .route("/todo/:id", delete(delete_todo).post(complete_todo));

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
async fn complete_todo(Path(id): Path<u32>) -> impl IntoResponse {
    let template = TodosTemplate { todos: [database::complete_todo(id as usize).expect("failed to complete todo item")].to_vec() };
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
