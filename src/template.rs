use crate::todo::{List, Task, Recipe, MealPlanEntry, RecipePhoto, RecipeWithPhoto, WeekDay};
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

#[derive(Template)]
#[template(path = "manage.html")] // Specify the path to the index.html template file
pub struct ManageTemplate {
    pub selected_list: usize,
    pub lists: Vec<List>,
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

// Recipe templates
#[derive(Template)]
#[template(path = "recipes.html")]
pub struct RecipesTemplate {
    pub recipes: Vec<RecipeWithPhoto>,
}

#[derive(Template)]
#[template(path = "recipe_detail.html")]
pub struct RecipeDetailTemplate {
    pub recipe: Recipe,
    pub photos: Vec<RecipePhoto>,
}

#[derive(Template)]
#[template(path = "recipe_form.html")]
pub struct RecipeFormTemplate {
    pub recipe: Option<Recipe>,
    pub is_edit: bool,
}

// Meal plan templates
#[derive(Template)]
#[template(path = "meal_plan.html")]
pub struct MealPlanTemplate {
    pub start_date: String,
    pub prev_week: String,
    pub next_week: String,
    pub week_days: Vec<WeekDay>,
}

#[derive(Template)]
#[template(path = "add_meal_form.html")]
pub struct AddMealFormTemplate {
    pub date: String,
}

#[derive(Template)]
#[template(path = "add_recipe_to_list.html")]
pub struct RecipeToListTemplate {
    pub recipe: Recipe,
    pub lists: Vec<List>,
}

#[derive(Template)]
#[template(path = "add_recipe_to_meal_plan.html")]
pub struct RecipeToMealPlanTemplate {
    pub recipe: Recipe,
}

#[derive(Template)]
#[template(path = "weekly_ingredients.html")]
pub struct WeeklyIngredientsTemplate {
    pub start_date: String,
    pub ingredients: Vec<String>,
    pub lists: Vec<List>,
}
