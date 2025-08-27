use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct Task {
    pub id: usize,
    pub text: String,
    pub completed: bool,
    pub list_id: usize,
    pub position: Option<i32>,
}

#[derive(Clone, Debug)]
pub struct List {
    pub id: usize,
    pub name: String,
}

#[derive(Deserialize)]
pub struct TaskForm {
    pub text: String,
}

#[derive(Deserialize)]
pub struct ListForm {
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct Recipe {
    pub id: usize,
    pub title: String,
    pub instructions: String,
    pub ingredients: String,
}

#[derive(Clone, Debug)]
pub struct MealPlanEntry {
    pub id: usize,
    pub date: String, // YYYY-MM-DD
    pub meal_text: String,
    pub recipe_id: Option<usize>,
}

#[derive(Deserialize)]
pub struct RecipeForm {
    pub title: String,
    pub instructions: String,
    pub ingredients: String,
}

#[derive(Deserialize)]
pub struct MealForm {
    pub meal_text: String,
    pub recipe_id: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct RecipePhoto {
    pub id: usize,
    pub recipe_id: usize,
    pub filename: String,
    pub original_name: String,
    pub file_size: i64,
    pub mime_type: String,
    pub upload_order: i32,
    pub thumbnail_blob: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct RecipeWithPhoto {
    pub recipe: Recipe,
    pub first_photo: Option<RecipePhoto>,
}

#[derive(Deserialize)]
pub struct RecipeToListForm {
    pub list_id: usize,
    #[serde(default)]
    pub ingredients: Vec<String>,
}

#[derive(Deserialize)]
pub struct RecipeToMealPlanForm {
    pub date: String,
    pub meal_text: Option<String>,
}

#[derive(Deserialize)]
pub struct WeeklyIngredientsForm {
    pub list_id: usize,
    #[serde(default)]
    pub ingredients: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct WeekDay {
    pub day_name: String,
    pub date: String,
    pub meals: Vec<MealPlanEntry>,
}
