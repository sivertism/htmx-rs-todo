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

use crate::todo::Todo;

#[derive(Template)]
#[template(path = "todos.html")]
pub struct TodosTemplate{
    // All fields passed in template can be used by Askama
    pub todos: Vec<Todo>
}

#[derive(Template)]
#[template(path = "todos-complete.html")]
pub struct TodosCompleteTemplate{
    // All fields passed in template can be used by Askama
    pub todo: Todo
}
