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
    #[serde(default, deserialize_with = "deserialize_optional_usize")]
    pub recipe_id: Option<usize>,
}

fn deserialize_optional_usize<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => s.parse().map(Some).map_err(serde::de::Error::custom),
        None => Ok(None),
    }
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
    pub date: String,          // Display format (dd.MM.yy)
    pub db_date: String,       // Database format (YYYY-MM-DD) for URLs
    pub meals: Vec<MealPlanEntry>,
}
