use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "index.html")] // Specify the path to the index.html template file
pub struct IndexTemplate {}

// A wrapper for turning askama templates into responses that can be handled by server
pub struct HtmlTemplate<T>(pub T);

impl<T> IntoResponse for HtmlTemplate<T>
where T: Template,
{
  fn into_response(self) -> Response {
    match self.0.render() {
      Ok(html) => Html(html).into_response(), // Success
      Err(err) => (
        StatusCode::INTERNAL_SERVER_ERROR, // Rendering failed
        format!("Failed to render template. Error: {}", err),
      ).into_response(),
    }
  }
}

use crate::todo::Task;

#[derive(Template)]
#[template(path = "tasks.html")]
pub struct TasksTemplate{
    // All fields passed in template can be used by Askama
    pub tasks: Vec<Task>
}

#[derive(Template)]
#[template(path = "tasks-complete.html")]
pub struct TasksCompleteTemplate{
    // All fields passed in template can be used by Askama
    pub task: Task
}
