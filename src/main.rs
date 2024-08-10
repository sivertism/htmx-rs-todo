mod template;
mod todo;

use axum::extract::{Path};
use axum::Form;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use tokio::net::TcpListener;
use todo::{Todo, TodoForm};
use template::{HtmlTemplate, IndexTemplate, TodosTemplate};

// Path to the JSON file for storing todos
const TODO_FILE_PATH: &str = "./src/todos.json";

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

// read our todos.json file
async fn read_todos() -> Vec<Todo> {
    let file = std::fs::read_to_string(TODO_FILE_PATH).expect("could not read todo file");
    serde_json::from_str(&file).expect("error parsing json")
}

// get todos handler
async fn get_todos() -> impl IntoResponse {
    let template = TodosTemplate { todos: read_todos().await};
    HtmlTemplate(template)
}

// delete todo handler
async fn delete_todo(Path(id): Path<u32>) -> impl IntoResponse {
    let mut todos = read_todos().await;

    // removes todo matching id from the routes path
    todos.retain(|todo| todo.id != id as usize);

    let file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(TODO_FILE_PATH)
        .unwrap();

    // writes over json with updated todo list
    serde_json::to_writer(file, &todos).unwrap();

    HtmlTemplate(TodosTemplate { todos })
}

pub async fn create_todo(
    form: Form<TodoForm>
    ) -> impl IntoResponse {
    let mut todos = read_todos().await;

    // create an id for our todos, a random or uuid would be better but this is fine
    let id = todos.len() as u32 + 1;

    // add new todo using data from our form
    todos.push(Todo {
        id: id as usize,
        text: form.text.clone(),
    });

    let file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(TODO_FILE_PATH)
        .unwrap();

    // writes over json with the updated todo list
    serde_json::to_writer(file, &todos).unwrap();

    HtmlTemplate(TodosTemplate { todos })
}
