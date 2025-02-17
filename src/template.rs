use crate::todo::{List, Task};
use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "index.html")] // Specify the path to the index.html template file
pub struct IndexTemplate {
    pub selected_list: usize,
    pub lists: Vec<List>,
    pub tasks: Vec<Task>,
}

// A wrapper for turning askama templates into responses that can be handled by server
pub struct HtmlTemplate<T>(pub T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(), // Success
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR, // Rendering failed
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}

#[derive(Template)]
#[allow(dead_code)]
#[template(path = "task.html")]
pub struct TaskTemplate {
    // All fields passed in template can be used by Askama
    pub task: Task,
}

#[derive(Template)]
#[allow(dead_code)]
#[template(path = "lists.html")]
pub struct ListsTemplate {
    // All fields passed in template can be used by Askama
    pub lists: Vec<List>,
}

#[derive(Template)]
#[allow(dead_code)]
#[template(path = "select_list.html")]
pub struct ListOptionsTemplate {
    // All fields passed in template can be used by Askama
    pub lists: Vec<List>,
    pub selected_list: usize,
}
