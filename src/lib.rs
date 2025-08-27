pub mod database;
pub mod template;
pub mod todo;

use axum::{
    extract::{Path, Query, State, Json, Multipart, DefaultBodyLimit, RawForm},
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Response, Html, Redirect},
    routing::{delete, get, post},
    Form,
};
use futures::stream::once;
use std::convert::Infallible;
use database::Database;
use reqwest::header;
use serde::Deserialize;
use anyhow::Context;
use template::*;
use askama::Template;
use todo::{ListForm, Task, TaskForm, RecipeForm, MealForm, RecipeToMealPlanForm, WeekDay};
use tracing::{info, warn};
use std::path::PathBuf;
use uuid::Uuid;
use image::ImageFormat;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub photos_dir: PathBuf,
}

#[derive(Deserialize)]
struct ListQuery {
    list_id: Option<usize>,
}

#[derive(Deserialize)]
struct ReorderPayload {
    order: Vec<u64>,
}

const HTMX_JS_GZIP: &[u8] = include_bytes!("../vendor/htmx.js.gz");
const SORTABLE_JS_GZIP: &[u8] = include_bytes!("../vendor/Sortable.js.gz");
const PICO_CSS_GZIP: &[u8] = include_bytes!("../vendor/pico.css.gz");

pub fn create_app(state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/", get(index))
        .route("/manage", get(manage).post(create_list))
        .route("/list/:id", delete(delete_list))
        .route("/task/:id", delete(delete_task).post(toggle_task))
        .route("/:list_id/task", post(create_task))
        .route("/create_list", post(create_list))
        .route("/reorder", post(reorder))
        .route("/recipes", get(recipes_page))
        .route("/recipes/new", get(new_recipe_form).post(create_recipe))
        .route("/recipes/:id", get(view_recipe))
        .route("/recipes/:id/edit", get(edit_recipe_form).post(update_recipe))
        .route("/recipes/:id/delete", post(delete_recipe))
        .route("/recipes/:id/add-to-list", get(recipe_to_list_form).post(add_recipe_to_list))
        .route("/recipes/:id/add-to-meal-plan", get(recipe_to_meal_plan_form).post(add_recipe_to_meal_plan))
        .route("/recipes/:id/upload-photos", post(upload_photos_unified))
        .route("/photos/:filename", get(serve_photo))
        .route("/thumbnails/:photo_id", get(serve_thumbnail))
        .route("/photos/default-recipe.svg", get(serve_default_photo))
        .route("/recipes/:id/photos/:photo_id/delete", post(delete_recipe_photo))
        .route("/meal-plan", get(meal_plan_page))
        .route("/meal-plan/:date/add", get(add_meal_form).post(add_meal))
        .route("/meal-plan/:id/delete", post(delete_meal))
        .route("/meal-plan/:start_date/add-ingredients", get(weekly_ingredients_form).post(add_weekly_ingredients))
        .route("/vendor/htmx.js", get(htmx))
        .route("/vendor/Sortable.js", get(sortable))
        .route("/vendor/pico.min.css", get(picocss))
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024)) // 50MB limit for photo uploads
        .with_state(state)
}

#[derive(Debug)]
struct CheckboxFormData {
    list_id: usize,
    ingredients: Vec<String>,
}

// Helper function to parse checkbox form data
fn parse_checkbox_form(body: &[u8]) -> CheckboxFormData {
    let form_data = std::str::from_utf8(body).unwrap_or("");
    let mut params = Vec::new();
    
    for pair in form_data.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            // Simple URL decode for + and %20 (space)
            let value = value.replace('+', " ").replace("%20", " ");
            params.push((key.to_string(), value));
        }
    }
    
    let mut list_id = 0;
    let mut ingredients = Vec::new();
    
    for (key, value) in params {
        match key.as_str() {
            "list_id" => {
                list_id = value.parse().unwrap_or(0);
            }
            "ingredients" => {
                ingredients.push(value);
            }
            _ => {}
        }
    }
    
    CheckboxFormData { list_id, ingredients }
}

// Helper function to determine the selected list
async fn determine_selected_list(list_query: &ListQuery, state: &AppState) -> usize {
    match list_query.list_id {
        Some(id) => id,
        None => {
            // Check if list 3 exists, otherwise use first available list
            match state.db.get_lists().await {
                Ok(lists) => {
                    if lists.iter().any(|list| list.id == 3) {
                        3
                    } else if !lists.is_empty() {
                        lists[0].id
                    } else {
                        3 // Fallback if no lists exist
                    }
                }
                _ => 3, // Fallback if error
            }
        }
    }
}

// Handler functions moved from main.rs
async fn index(list_query: Query<ListQuery>, State(state): State<AppState>) -> impl IntoResponse {
    let selected_list = determine_selected_list(&list_query, &state).await;

    let lists = state.db.get_lists().await.expect("Get list options");


    info!("Getting tasks for list_id={}", selected_list);
    if let Ok(tasks) = state.db.get_tasks(selected_list as usize).await {
        let incomplete : Vec<Task>= tasks.clone().into_iter().filter(|task| !task.completed).collect();
        info!(
            "Got incomplete tasks: {:?} from list with id {:?}",
            incomplete, selected_list
        );
        let template = IndexTemplate { selected_list, lists, tasks};
        HtmlTemplate(template).into_response()
    } else {
        warn!("Failed to get tasks for list_id={}", selected_list);
        let tasks: Vec<Task> = vec![];
        let template = IndexTemplate { selected_list, lists, tasks};
        HtmlTemplate(template).into_response()
    }
}

async fn delete_task(State(state): State<AppState>, Path(id): Path<u32>) -> StatusCode {
    state.db.delete_task(id as usize).await.expect("Delete task");
    info!("Deleted task with id {}", id);
    StatusCode::OK
}

async fn delete_list(State(state): State<AppState>, Path(id): Path<u32>) -> impl IntoResponse {
    state.db.delete_list(id as usize).await.expect("Delete list");
    info!("Deleted list with id {}", id);

    // Need to use HX-Redirect to force redirect when using HTMX
    let mut headers = HeaderMap::new();
    headers.insert("HX-Redirect", "/manage".parse().unwrap());
    (StatusCode::SEE_OTHER, headers, "").into_response()
}

async fn toggle_task(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    let id = id as usize;
    info!("Toggling task with id {}", id);
    if let Ok(_) = state.db.toggle_task_completed(id).await {
        if let Ok(task) = state.db.get_task(id).await {
            return HtmlTemplate(TaskTemplate { task }).into_response();
        } else {
            warn!("Toggled task with id {}, but failed to retrieve it!", id);
            StatusCode::OK.into_response()
        }
    } else {
        warn!("Failed to toggle task with id {}", id);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

async fn create_task(
    State(state): State<AppState>,
    Path(list_id): Path<u32>,
    form: Form<TaskForm>,
) -> impl IntoResponse {
    let text = form.text.clone();
    info!("Inserting task item with list_id {}", list_id);

    let id = state.db.create_task(text, list_id as usize).await.expect("Create task on db");

    info!("Task item with id {} created", id);

    let task = Task {
        id: id,
        text: form.text.clone(),
        completed: false,
        list_id: list_id as usize,
        position: None, // Will be set by database
    };

    // could just return one task if we fix the template to only add an item!
    HtmlTemplate(TaskTemplate { task })
}

async fn create_list(State(state): State<AppState>, form: Form<ListForm>) -> Response {
    let name = form.name.clone();

    if let Ok(id) = state.db.create_list(name.clone()).await.context("Create list") {
        info!("List item with id {} created", id);
        return Html(format!(r#"<option class="select-list" value="?list_id={id}">{name}</option>"#)).into_response();
    } else {
        warn!("Failed to create list");
        return StatusCode::BAD_REQUEST.into_response();
    }
}

async fn manage(list_query: Query<ListQuery>, State(state): State<AppState>) -> impl IntoResponse {
    let selected_list = determine_selected_list(&list_query, &state).await;

    let lists = state.db.get_lists().await.expect("Get list options");
    let template = ManageTemplate { selected_list, lists};
    HtmlTemplate(template).into_response()
}

async fn htmx() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/javascript".parse().unwrap());
    headers.insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
    (headers, HTMX_JS_GZIP)
}

async fn sortable() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/javascript".parse().unwrap());
    headers.insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
    (headers, SORTABLE_JS_GZIP)
}

async fn picocss() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "text/css".parse().unwrap());
    headers.insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
    (headers, PICO_CSS_GZIP)
}

async fn reorder(State(state): State<AppState>,
                 Query(params): Query<ListQuery>, 
                 Json(payload): Json<ReorderPayload>,
                 ) -> StatusCode {
    println!("List {:?} reordered to {:?}", params.list_id, payload.order);
    match state.db.reorder(params.list_id.unwrap(), payload.order).await {
        Ok(()) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// Recipe handlers
async fn recipes_page(State(state): State<AppState>) -> impl IntoResponse {
    let recipes = state.db.get_recipes().await.unwrap_or_default();
    
    // Get first photo for each recipe
    let mut recipes_with_photos = Vec::new();
    for recipe in recipes {
        let first_photo = state.db.get_recipe_first_photo(recipe.id).await.unwrap_or(None);
        recipes_with_photos.push(todo::RecipeWithPhoto {
            recipe,
            first_photo,
        });
    }
    
    let template = RecipesTemplate { recipes: recipes_with_photos };
    HtmlTemplate(template).into_response()
}

async fn new_recipe_form() -> impl IntoResponse {
    let template = RecipeFormTemplate { 
        recipe: None, 
        is_edit: false 
    };
    HtmlTemplate(template).into_response()
}

// Helper function to parse recipe form data
async fn parse_recipe_multipart(mut multipart: Multipart) -> Result<(String, String, String, Vec<PhotoData>), StatusCode> {
    let mut title = String::new();
    let mut instructions = String::new();
    let mut ingredients = String::new();
    let mut photos = Vec::new();
    let limits = PhotoUploadLimits::default();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        
        match name.as_str() {
            "title" => {
                if let Ok(value) = field.text().await {
                    title = value;
                }
            }
            "instructions" => {
                if let Ok(value) = field.text().await {
                    instructions = value;
                }
            }
            "ingredients" => {
                if let Ok(value) = field.text().await {
                    ingredients = value;
                }
            }
            "photos" => {
                if let Some(filename) = field.file_name() {
                    let filename = filename.to_string();
                    if let Some(content_type) = field.content_type() {
                        let content_type = content_type.to_string();
                        if is_supported_image_type(&content_type) {
                            if let Ok(data) = field.bytes().await {
                                if data.len() <= limits.max_file_size && data.len() > 0 {
                                    photos.push(PhotoData {
                                        filename,
                                        content_type,
                                        data,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if title.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok((title, instructions, ingredients, photos))
}

async fn create_recipe(
    State(state): State<AppState>,
    multipart: Multipart,
) -> impl IntoResponse {
    // Parse multipart form data
    let (title, instructions, ingredients, photos) = match parse_recipe_multipart(multipart).await {
        Ok(data) => data,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid form data or missing title").into_response(),
    };

    // Create recipe in database
    let recipe_id = match state.db.create_recipe(title, instructions, ingredients).await {
        Ok(id) => id,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    // Process uploaded photos
    let mut uploaded_count = 0;
    let limits = PhotoUploadLimits::default();
    
    for photo in photos {
        if uploaded_count >= limits.max_photos {
            break;
        }

        match save_photo_to_disk_and_db(photo, recipe_id, &state).await {
            Ok(_) => {
                uploaded_count += 1;
                info!("Photo uploaded for new recipe, total: {}", uploaded_count);
            }
            Err(e) => {
                warn!("Failed to save photo for new recipe: {}", e);
            }
        }
    }

    Redirect::to("/recipes").into_response()
}

async fn view_recipe(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    match state.db.get_recipe(id as usize).await {
        Ok(recipe) => {
            let photos = state.db.get_recipe_photos(id as usize).await.unwrap_or_default();
            let template = RecipeDetailTemplate { recipe, photos };
            HtmlTemplate(template).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response()
    }
}

async fn edit_recipe_form(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    match state.db.get_recipe(id as usize).await {
        Ok(recipe) => {
            let template = RecipeFormTemplate { 
                recipe: Some(recipe), 
                is_edit: true 
            };
            HtmlTemplate(template).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response()
    }
}

async fn update_recipe(
    State(state): State<AppState>,
    Path(id): Path<u32>,
    form: Form<RecipeForm>,
) -> impl IntoResponse {
    match state.db.update_recipe(
        id as usize,
        form.title.clone(),
        form.instructions.clone(),
        form.ingredients.clone(),
    ).await {
        Ok(_) => {
            Redirect::to(&format!("/recipes/{}", id)).into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

async fn delete_recipe(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    match state.db.delete_recipe(id as usize).await {
        Ok(_) => {
            let mut headers = HeaderMap::new();
            headers.insert("HX-Redirect", "/recipes".parse().unwrap());
            (headers, "").into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

async fn recipe_to_list_form(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    let recipe = match state.db.get_recipe(id as usize).await {
        Ok(recipe) => recipe,
        Err(_) => return StatusCode::NOT_FOUND.into_response()
    };
    
    let lists = state.db.get_lists().await.unwrap_or_default();
    
    let template = RecipeToListTemplate { recipe, lists };
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

async fn add_recipe_to_list(
    State(state): State<AppState>,
    Path(id): Path<u32>,
    RawForm(body): RawForm,
) -> impl IntoResponse {
    // Get recipe to validate it exists
    let _recipe = match state.db.get_recipe(id as usize).await {
        Ok(recipe) => recipe,
        Err(_) => return StatusCode::NOT_FOUND.into_response()
    };
    
    // Parse form data using helper function
    let parsed_data = parse_checkbox_form(&body);
    let list_id = parsed_data.list_id;
    let ingredients = parsed_data.ingredients;
    
    // Add each selected ingredient as a task
    for ingredient in &ingredients {
        if !ingredient.trim().is_empty() {
            let _ = state.db.create_task(
                ingredient.trim().to_string(),
                list_id
            ).await;
        }
    }
    
    Redirect::to(&format!("/recipes/{}", id)).into_response()
}

async fn recipe_to_meal_plan_form(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    let recipe = match state.db.get_recipe(id as usize).await {
        Ok(recipe) => recipe,
        Err(_) => return StatusCode::NOT_FOUND.into_response()
    };
    
    let template = RecipeToMealPlanTemplate { recipe };
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

async fn add_recipe_to_meal_plan(
    State(state): State<AppState>,
    Path(id): Path<u32>,
    form: Form<RecipeToMealPlanForm>,
) -> impl IntoResponse {
    // Get recipe to validate it exists and get its title
    let recipe = match state.db.get_recipe(id as usize).await {
        Ok(recipe) => recipe,
        Err(_) => return StatusCode::NOT_FOUND.into_response()
    };
    
    // Use provided meal text or fall back to recipe title
    let meal_text = form.meal_text.clone()
        .filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| recipe.title.clone());
    
    // Add recipe to meal plan for the specified date
    match state.db.create_meal_plan_entry(
        form.date.clone(),
        meal_text,
        Some(id as usize),
    ).await {
        Ok(_) => Redirect::to("/meal-plan").into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

// Meal plan helpers
fn get_week_start_monday() -> chrono::NaiveDate {
    use chrono::{Utc, Datelike, Duration};
    let today = Utc::now().date_naive();
    let days_since_monday = today.weekday().num_days_from_monday();
    today - Duration::days(days_since_monday as i64)
}

fn parse_week_start_date(week_param: Option<String>) -> chrono::NaiveDate {
    use chrono::NaiveDate;
    match week_param {
        Some(week_str) => {
            NaiveDate::parse_from_str(&week_str, "%Y-%m-%d")
                .unwrap_or_else(|_| get_week_start_monday())
        }
        None => get_week_start_monday(),
    }
}

fn build_week_structure(start_date: chrono::NaiveDate, meals_by_date: std::collections::HashMap<String, Vec<crate::todo::MealPlanEntry>>) -> Vec<WeekDay> {
    use chrono::Duration;
    let day_names = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
    let mut week_days = Vec::new();
    let mut meals_map = meals_by_date;
    
    for i in 0..7 {
        let date = start_date + Duration::days(i);
        let date_str = date.format("%Y-%m-%d").to_string();
        let day_name = day_names[i as usize].to_string();
        let meals = meals_map.remove(&date_str).unwrap_or_default();
        
        week_days.push(WeekDay {
            day_name,
            date: date_str,
            meals,
        });
    }
    
    week_days
}

// Meal plan handlers
async fn meal_plan_page(Query(params): Query<WeekQuery>, State(state): State<AppState>) -> impl IntoResponse {
    use chrono::Duration;
    use std::collections::HashMap;
    
    let start_date = parse_week_start_date(params.week);
    let start_date_str = start_date.format("%Y-%m-%d").to_string();
    let prev_week = (start_date - Duration::days(7)).format("%Y-%m-%d").to_string();
    let next_week = (start_date + Duration::days(7)).format("%Y-%m-%d").to_string();
    
    // Get all meals for this week and group by date
    let meal_plan = state.db.get_meal_plan_for_week(start_date_str.clone()).await.unwrap_or_default();
    let mut meals_by_date: HashMap<String, Vec<_>> = HashMap::new();
    for meal in meal_plan {
        meals_by_date.entry(meal.date.clone()).or_default().push(meal);
    }
    
    let week_days = build_week_structure(start_date, meals_by_date);
    
    let template = MealPlanTemplate { 
        start_date: start_date_str,
        prev_week,
        next_week,
        week_days,
    };
    HtmlTemplate(template).into_response()
}

async fn add_meal_form(
    State(state): State<AppState>,
    Path(date): Path<String>,
) -> impl IntoResponse {
    match state.db.get_recipes().await {
        Ok(recipes) => {
            let template = AddMealFormTemplate { 
                date,
                recipes
            };
            HtmlTemplate(template).into_response()
        }
        Err(_) => {
            let template = AddMealFormTemplate { 
                date,
                recipes: vec![]
            };
            HtmlTemplate(template).into_response()
        }
    }
}

async fn add_meal(
    State(state): State<AppState>,
    Path(date): Path<String>,
    form: Form<MealForm>,
) -> impl IntoResponse {
    match state.db.create_meal_plan_entry(
        date,
        form.meal_text.clone(),
        form.recipe_id,
    ).await {
        Ok(_) => {
            Redirect::to("/meal-plan").into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

async fn delete_meal(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    match state.db.delete_meal_plan_entry(id as usize).await {
        Ok(_) => {
            let mut headers = HeaderMap::new();
            headers.insert("HX-Redirect", "/meal-plan".parse().unwrap());
            (StatusCode::SEE_OTHER, headers, "").into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

#[derive(Deserialize)]
struct WeekQuery {
    week: Option<String>,
}

async fn weekly_ingredients_form(
    State(state): State<AppState>,
    Path(start_date): Path<String>,
) -> impl IntoResponse {
    use chrono::NaiveDate;
    
    // Parse start date and calculate week range
    let _start = NaiveDate::parse_from_str(&start_date, "%Y-%m-%d")
        .unwrap_or_else(|_| chrono::Utc::now().date_naive());
    
    // Get all meal plan entries for this week that have recipes
    let meal_entries = state.db.get_meal_plan_for_week(start_date.clone()).await.unwrap_or_default();
    
    // Collect unique ingredients from all recipes
    let mut all_ingredients = Vec::new();
    for entry in meal_entries {
        if let Some(recipe_id) = entry.recipe_id {
            if let Ok(recipe) = state.db.get_recipe(recipe_id).await {
                for ingredient in recipe.ingredients.split('\n') {
                    let ingredient = ingredient.trim();
                    if !ingredient.is_empty() && !all_ingredients.contains(&ingredient.to_string()) {
                        all_ingredients.push(ingredient.to_string());
                    }
                }
            }
        }
    }
    
    let lists = state.db.get_lists().await.unwrap_or_default();
    let template = WeeklyIngredientsTemplate {
        start_date,
        ingredients: all_ingredients,
        lists,
    };
    
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

async fn add_weekly_ingredients(
    State(state): State<AppState>,
    Path(_start_date): Path<String>,
    RawForm(body): RawForm,
) -> impl IntoResponse {
    // Parse form data using helper function
    let parsed_data = parse_checkbox_form(&body);
    let list_id = parsed_data.list_id;
    let ingredients = parsed_data.ingredients;
    
    // Add each selected ingredient as a task
    for ingredient in &ingredients {
        if !ingredient.trim().is_empty() {
            let _ = state.db.create_task(
                ingredient.trim().to_string(),
                list_id
            ).await;
        }
    }
    
    Redirect::to("/meal-plan").into_response()
}

// Photo handling utilities
fn generate_thumbnail(image_data: &[u8], max_size: u32) -> anyhow::Result<Vec<u8>> {
    use anyhow::Context;
    
    let img = image::load_from_memory(image_data)
        .context("Failed to load image from memory")?;
    let thumbnail = img.thumbnail(max_size, max_size);
    
    let mut buffer = Vec::new();
    thumbnail.write_to(&mut std::io::Cursor::new(&mut buffer), ImageFormat::Jpeg)
        .context("Failed to write thumbnail to buffer")?;
    Ok(buffer)
}


fn is_supported_image_type(mime_type: &str) -> bool {
    matches!(mime_type, "image/jpeg" | "image/jpg" | "image/png" | "image/webp")
}

// Photo upload helper functions
#[derive(Debug)]
struct PhotoData {
    filename: String,
    content_type: String,
    data: bytes::Bytes,
}

struct PhotoUploadLimits {
    max_photos: usize,
    max_file_size: usize,
}

impl Default for PhotoUploadLimits {
    fn default() -> Self {
        Self {
            max_photos: 10,
            max_file_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

async fn validate_photo_upload_preconditions(
    state: &AppState,
    recipe_id: usize,
    limits: &PhotoUploadLimits,
) -> Result<usize, StatusCode> {
    // Check recipe exists
    if state.db.get_recipe(recipe_id).await.is_err() {
        warn!("Recipe {} not found", recipe_id);
        return Err(StatusCode::NOT_FOUND);
    }

    // Get current photo count
    let existing_photos = state.db.get_recipe_photos(recipe_id).await.unwrap_or_default();
    if existing_photos.len() >= limits.max_photos {
        warn!("Recipe {} already has max photos: {}", recipe_id, existing_photos.len());
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(existing_photos.len())
}

async fn parse_multipart_photos(
    body: bytes::Bytes, 
    content_type: &str,
    limits: &PhotoUploadLimits,
) -> Vec<PhotoData> {
    let stream = once(async move { Result::<bytes::Bytes, Infallible>::Ok(body) });
    
    let boundary = match multer::parse_boundary(content_type) {
        Ok(boundary) => boundary,
        Err(e) => {
            warn!("Failed to parse multipart boundary: {}", e);
            return vec![];
        }
    };

    let mut multipart = multer::Multipart::new(stream, boundary);
    let mut photos = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name != "photos" {
            continue;
        }

        let filename = field.file_name().map(|s| s.to_string()).unwrap_or_else(|| "upload".to_string());
        let field_content_type = field.content_type()
            .map(|mime| mime.as_ref())
            .unwrap_or("application/octet-stream")
            .to_string();
        
        if !is_supported_image_type(&field_content_type) {
            warn!("Unsupported content type: {}", field_content_type);
            continue;
        }

        match field.bytes().await {
            Ok(data) if data.len() <= limits.max_file_size && data.len() > 0 => {
                photos.push(PhotoData {
                    filename,
                    content_type: field_content_type,
                    data,
                });
            }
            Ok(data) if data.len() == 0 => {
                info!("Empty photo data, skipping");
            }
            Ok(data) => {
                warn!("Photo too large: {} bytes (max: {})", data.len(), limits.max_file_size);
            }
            Err(e) => {
                warn!("Failed to read photo data: {}", e);
            }
        }
    }

    photos
}

fn parse_direct_image_upload(body: bytes::Bytes, content_type: &str, limits: &PhotoUploadLimits) -> Vec<PhotoData> {
    if body.len() <= limits.max_file_size && body.len() > 0 {
        vec![PhotoData {
            filename: "mobile_upload".to_string(),
            content_type: content_type.to_string(),
            data: body,
        }]
    } else {
        warn!("Direct image too large or empty: {} bytes", body.len());
        vec![]
    }
}

fn get_file_extension_from_content_type(content_type: &str) -> &str {
    match content_type {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        _ => "jpg",
    }
}

async fn save_photo_to_disk_and_db(
    photo: PhotoData,
    recipe_id: usize,
    state: &AppState,
) -> Result<(), anyhow::Error> {
    // Ensure photos directory exists
    std::fs::create_dir_all(&state.photos_dir)
        .context("Failed to create photos directory")?;

    // Generate unique filename
    let file_extension = get_file_extension_from_content_type(&photo.content_type);
    let unique_filename = format!("{}.{}", Uuid::new_v4(), file_extension);
    let file_path = state.photos_dir.join(&unique_filename);

    // Save file to disk
    std::fs::write(&file_path, &photo.data)
        .context("Failed to write photo to disk")?;

    // Generate thumbnail
    let thumbnail = generate_thumbnail(&photo.data, 200).ok();

    // Get next order and save to database
    let upload_order = state.db.get_next_photo_order(recipe_id).await.unwrap_or(0);
    
    state.db.create_recipe_photo(
        recipe_id,
        unique_filename,
        photo.filename,
        photo.data.len() as i64,
        photo.content_type,
        upload_order,
        thumbnail,
    ).await
    .context("Failed to save photo to database")?;

    Ok(())
}

// Simplified unified photo upload handler
async fn upload_photos_unified(
    State(state): State<AppState>,
    Path(recipe_id): Path<u32>,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> impl IntoResponse {
    let recipe_id = recipe_id as usize;
    let limits = PhotoUploadLimits::default();

    // Validate preconditions
    let existing_photo_count = match validate_photo_upload_preconditions(&state, recipe_id, &limits).await {
        Ok(count) => count,
        Err(status) => return status.into_response(),
    };

    // Get content type and parse photos
    let content_type = headers.get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let photos_data = if content_type.starts_with("multipart") {
        parse_multipart_photos(body, content_type, &limits).await
    } else if content_type.starts_with("image/") {
        parse_direct_image_upload(body, content_type, &limits)
    } else {
        warn!("Unsupported content type: {}", content_type);
        return (StatusCode::BAD_REQUEST, "Unsupported content type").into_response();
    };

    // Process and save photos
    let mut uploaded_count = 0;
    for photo in photos_data {
        if uploaded_count + existing_photo_count >= limits.max_photos {
            break;
        }

        match save_photo_to_disk_and_db(photo, recipe_id, &state).await {
            Ok(_) => {
                uploaded_count += 1;
                info!("Photo uploaded successfully, total: {}", uploaded_count);
            }
            Err(e) => {
                warn!("Failed to save photo: {}", e);
            }
        }
    }

    info!("Upload complete. Total uploaded: {}", uploaded_count);
    
    if uploaded_count > 0 {
        Redirect::to(&format!("/recipes/{}", recipe_id)).into_response()
    } else {
        (StatusCode::BAD_REQUEST, "No valid photos uploaded").into_response()
    }
}


// Serve full-size photos
async fn serve_photo(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> impl IntoResponse {
    let file_path = state.photos_dir.join(&filename);
    
    match tokio::fs::read(&file_path).await {
        Ok(data) => {
            let content_type = if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
                "image/jpeg"
            } else if filename.ends_with(".png") {
                "image/png"
            } else if filename.ends_with(".webp") {
                "image/webp"
            } else {
                "application/octet-stream"
            };
            
            let mut headers = HeaderMap::new();
            headers.insert("content-type", content_type.parse().unwrap());
            headers.insert("cache-control", "public, max-age=86400".parse().unwrap());
            
            (headers, data).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response()
    }
}

// Serve thumbnails from database
async fn serve_thumbnail(
    State(state): State<AppState>,
    Path(photo_id): Path<u32>,
) -> impl IntoResponse {
    match state.db.get_recipe_photo_by_id(photo_id as usize).await {
        Ok(Some(photo)) => {
            if let Some(thumbnail_data) = photo.thumbnail_blob {
                let mut headers = HeaderMap::new();
                headers.insert("content-type", "image/jpeg".parse().unwrap());
                headers.insert("cache-control", "public, max-age=86400".parse().unwrap());
                
                (headers, thumbnail_data).into_response()
            } else {
                StatusCode::NOT_FOUND.into_response()
            }
        }
        _ => StatusCode::NOT_FOUND.into_response()
    }
}

// Serve default photo
async fn serve_default_photo(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let default_path = state.photos_dir.join("default-recipe.svg");
    
    match tokio::fs::read(&default_path).await {
        Ok(data) => {
            let mut headers = HeaderMap::new();
            headers.insert("content-type", "image/svg+xml".parse().unwrap());
            headers.insert("cache-control", "public, max-age=86400".parse().unwrap());
            
            (headers, data).into_response()
        }
        Err(_) => {
            // Fallback: simple SVG response with hex colors as string literals
            let svg = format!(
                r#"<svg width="400" height="300" xmlns="http://www.w3.org/2000/svg"><rect width="400" height="300" fill="{}"/><text x="200" y="150" text-anchor="middle" fill="{}" font-family="sans-serif" font-size="16">No Photo</text></svg>"#,
                "#64748b", "#f1f5f9"
            );
            let mut headers = HeaderMap::new();
            headers.insert("content-type", "image/svg+xml".parse().unwrap());
            (headers, svg.as_bytes().to_vec()).into_response()
        }
    }
}

// Delete individual photo
async fn delete_recipe_photo(
    State(state): State<AppState>,
    Path((recipe_id, photo_id)): Path<(u32, u32)>,
) -> impl IntoResponse {
    let photo_id = photo_id as usize;
    
    // Get photo info first to delete the file
    if let Ok(Some(photo)) = state.db.get_recipe_photo_by_id(photo_id).await {
        // Delete file from disk
        let file_path = state.photos_dir.join(&photo.filename);
        let _ = tokio::fs::remove_file(file_path).await;
        
        // Delete from database
        if state.db.delete_recipe_photo(photo_id).await.is_ok() {
            Redirect::to(&format!("/recipes/{}", recipe_id)).into_response()
        } else {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}