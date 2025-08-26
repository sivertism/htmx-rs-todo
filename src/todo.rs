use serde::Deserialize;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Task {
    pub id: usize,
    pub text: String,
    pub completed: bool,
    pub list_id: usize,
    pub position: Option<i32>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct List {
    pub id: usize,
    pub name: String,
}

// Don't worry about this for now, will use later
#[allow(dead_code)]
#[derive(Deserialize)]
pub struct TaskForm {
    pub text: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct ListForm {
    pub name: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Recipe {
    pub id: usize,
    pub title: String,
    pub instructions: String,
    pub ingredients: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MealPlanEntry {
    pub id: usize,
    pub date: String, // YYYY-MM-DD
    pub meal_text: String,
    pub recipe_id: Option<usize>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct RecipeForm {
    pub title: String,
    pub instructions: String,
    pub ingredients: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct MealForm {
    pub meal_text: String,
    pub recipe_id: Option<usize>,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct RecipeWithPhoto {
    pub recipe: Recipe,
    pub first_photo: Option<RecipePhoto>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct RecipeToListForm {
    pub list_id: usize,
    #[serde(default)]
    pub ingredients: Vec<String>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct RecipeToMealPlanForm {
    pub date: String,
    pub meal_text: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct WeeklyIngredientsForm {
    pub list_id: usize,
    #[serde(default)]
    pub ingredients: Vec<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct WeekDay {
    pub day_name: String,
    pub date: String,
    pub meals: Vec<MealPlanEntry>,
}
