mod template;
mod todo;
mod database;

use axum::extract::{Path, Query};
use axum::Form;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, get_service};
use axum::http::StatusCode;
use tokio::net::TcpListener;
use todo::{Task,TaskForm, ListForm};
use template::{HtmlTemplate, IndexTemplate, TasksTemplate, TasksCompleteTemplate};
use tower_http::services::ServeFile;

#[tokio::main]
async fn main() -> std::io::Result<()> {

    database::init_db();

    let app = axum::Router::new() // Create a new Axum Router
            .route("/", get(index)) // Define a GET route for the root path, handled by the
                                     // `index` function
            .route("/task", get(get_tasks).post(create_task))
            .route("/completed", get(get_completed).post(toggle_task))
            // :id defines path parameters for our route
            .route("/task/:id", delete(delete_task).post(toggle_task))
            .route("/create_list", get_service(ServeFile::new("templates/create_list.html")).post(create_list));

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

// get tasks handler
async fn get_tasks() -> impl IntoResponse {
    let tasks =  database::get_tasks(false).expect("failed to get tasks");
    println!("Got tasks: {:?}", tasks);
    let template = TasksTemplate {tasks: tasks};
    HtmlTemplate(template)
}

async fn get_completed() -> impl IntoResponse {
    let tasks =  database::get_tasks(true).expect("failed to get tasks");
    println!("Got tasks: {:?}", tasks);
    let template = TasksTemplate {tasks: tasks};
    HtmlTemplate(template)
}

// delete task handler
async fn delete_task(Path(id): Path<u32>) -> StatusCode {
    database::delete_task(id as usize);
    //HtmlTemplate(TasksTemplate { tasks })
    StatusCode::OK
}

// complete tasks handler
async fn toggle_task(Path(id): Path<u32>) -> impl IntoResponse {
    let template = TasksCompleteTemplate { task: database::toggle_task(id as usize).expect("failed to complete task item")};
    HtmlTemplate(template)
}

pub async fn create_task(
    form: Form<TaskForm>
    ) -> impl IntoResponse {
    let id = database::create_task(form.text.clone());
    let tasks = vec![Task{id: id, text: form.text.clone(), completed: false}];
    println!("Task item with id {} created", id);

    // could just return one task if we fix the template to only add an item!
    HtmlTemplate(TasksTemplate {tasks})
}

pub async fn create_list(
    form: Form<ListForm>
    ) -> impl IntoResponse {
    let id = database::create_task(form.text.clone());
    let tasks = vec![Task{id: id, text: form.text.clone(), completed: false}];
    println!("Task item with id {} created", id);

    // could just return one task if we fix the template to only add an item!
    HtmlTemplate(TasksTemplate {tasks})
}
