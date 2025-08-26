use axum_test::{TestServer, multipart::MultipartForm};
use tempfile::TempDir;
use chrono::{Utc, Datelike, Duration, NaiveDate};

mod common;
use common::*;

#[tokio::test]
async fn test_meal_plan_page_loads() {
    let (server, _temp_dir) = setup_test_server().await;
    
    let response = server.get("/meal-plan").await;
    
    response.assert_status_ok();
    response.assert_text_contains("Meal Plan");
    response.assert_text_contains("Week of");
    response.assert_text_contains("Previous Week");
    response.assert_text_contains("Next Week");
    response.assert_text_contains("Monday");
    response.assert_text_contains("Sunday");
}

#[tokio::test]
async fn test_meal_plan_shows_all_days() {
    let (server, _temp_dir) = setup_test_server().await;
    
    let response = server.get("/meal-plan").await;
    
    response.assert_status_ok();
    
    // Check all 7 days are shown
    let days = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
    for day in days {
        response.assert_text_contains(day);
    }
    
    // Check that "Add Meal" buttons are present
    response.assert_text_contains("+ Add Meal");
}

#[tokio::test]
async fn test_meal_plan_week_navigation() {
    let (server, _temp_dir) = setup_test_server().await;
    
    // Get current week start date
    let today = Utc::now().date_naive();
    let days_since_monday = today.weekday().num_days_from_monday();
    let current_week_start = today - Duration::days(days_since_monday as i64);
    let next_week_start = current_week_start + Duration::days(7);
    let prev_week_start = current_week_start - Duration::days(7);
    
    // Test next week navigation
    let response = server.get(&format!("/meal-plan?week={}", next_week_start.format("%Y-%m-%d"))).await;
    response.assert_status_ok();
    response.assert_text_contains(&format!("Week of {}", next_week_start.format("%Y-%m-%d")));
    
    // Test previous week navigation
    let response = server.get(&format!("/meal-plan?week={}", prev_week_start.format("%Y-%m-%d"))).await;
    response.assert_status_ok();
    response.assert_text_contains(&format!("Week of {}", prev_week_start.format("%Y-%m-%d")));
}

#[tokio::test]
async fn test_add_meal_form_loads() {
    let (server, _temp_dir) = setup_test_server().await;
    
    let today = Utc::now().date_naive().format("%Y-%m-%d");
    let response = server.get(&format!("/meal-plan/{}/add", today)).await;
    
    response.assert_status_ok();
    response.assert_text_contains("Add Meal for");
    response.assert_text_contains("Add from Recipe");
    response.assert_text_contains("Custom Meal");
    response.assert_text_contains("No recipes available");
}

#[tokio::test]
async fn test_add_meal_form_with_recipes() {
    let (server, _temp_dir) = setup_test_server_with_recipe().await;
    
    let today = Utc::now().date_naive().format("%Y-%m-%d");
    let response = server.get(&format!("/meal-plan/{}/add", today)).await;
    
    response.assert_status_ok();
    response.assert_text_contains("Add Meal for");
    response.assert_text_contains("Test Recipe");
    response.assert_text_contains("Choose a recipe");
}

#[tokio::test]
async fn test_add_custom_meal() {
    let (server, _temp_dir) = setup_test_server().await;
    
    let today = Utc::now().date_naive().format("%Y-%m-%d");
    let response = server
        .post(&format!("/meal-plan/{}/add", today))
        .form(&serde_json::json!({
            "meal_text": "Leftover pizza"
        }))
        .await;
    
    response.assert_status_see_other(); // Should redirect to meal plan
    
    // Check that meal appears on meal plan
    let response = server.get("/meal-plan").await;
    response.assert_status_ok();
    response.assert_text_contains("Leftover pizza");
}

#[tokio::test]
async fn test_add_recipe_meal() {
    let (server, _temp_dir) = setup_test_server_with_recipe().await;
    
    let today = Utc::now().date_naive().format("%Y-%m-%d");
    let response = server
        .post(&format!("/meal-plan/{}/add", today))
        .form(&serde_json::json!({
            "meal_text": "Test Recipe",
            "recipe_id": "1"
        }))
        .await;
    
    response.assert_status_see_other(); // Should redirect to meal plan
    
    // Check that meal appears on meal plan with recipe link
    let response = server.get("/meal-plan").await;
    response.assert_status_ok();
    response.assert_text_contains("Test Recipe");
    response.assert_text_contains("/recipes/1"); // Should be a link
}

#[tokio::test]
async fn test_delete_meal() {
    let (server, _temp_dir) = setup_test_server_with_meal().await;
    
    // Delete the meal (assuming it gets ID 1)
    let response = server.post("/meal-plan/1/delete").await;
    
    response.assert_status_see_other(); // Should redirect
    response.assert_header("HX-Redirect", "/meal-plan");
    
    // Check that meal is gone
    let response = server.get("/meal-plan").await;
    response.assert_status_ok();
    let body = response.text();
    assert!(!body.contains("Test Meal"));
}

#[tokio::test]
async fn test_multiple_meals_per_day() {
    let (server, _temp_dir) = setup_test_server().await;
    
    let today = Utc::now().date_naive().format("%Y-%m-%d");
    
    // Add multiple meals to the same day
    let meals = ["Breakfast: Oatmeal", "Lunch: Sandwich", "Dinner: Pasta"];
    
    for meal in meals {
        let _ = server
            .post(&format!("/meal-plan/{}/add", today))
            .form(&serde_json::json!({
                "meal_text": meal
            }))
            .await;
    }
    
    // Check all meals appear
    let response = server.get("/meal-plan").await;
    response.assert_status_ok();
    
    for meal in meals {
        response.assert_text_contains(meal);
    }
}

#[tokio::test]
async fn test_meal_plan_empty_days() {
    let (server, _temp_dir) = setup_test_server().await;
    
    let response = server.get("/meal-plan").await;
    
    response.assert_status_ok();
    response.assert_text_contains("No meals planned");
}

#[tokio::test]
async fn test_meal_plan_different_weeks() {
    let (server, _temp_dir) = setup_test_server().await;
    
    // Add a meal to today
    let today = Utc::now().date_naive();
    let response = server
        .post(&format!("/meal-plan/{}/add", today.format("%Y-%m-%d")))
        .form(&serde_json::json!({
            "meal_text": "Today's Meal"
        }))
        .await;
    response.assert_status_see_other();
    
    // Add a meal to next week
    let next_week_day = today + Duration::days(7);
    let response = server
        .post(&format!("/meal-plan/{}/add", next_week_day.format("%Y-%m-%d")))
        .form(&serde_json::json!({
            "meal_text": "Next Week's Meal"
        }))
        .await;
    response.assert_status_see_other();
    
    // Check current week shows only current week's meal
    let response = server.get("/meal-plan").await;
    response.assert_status_ok();
    response.assert_text_contains("Today&#x27;s Meal");
    let body = response.text();
    assert!(!body.contains("Next Week&#x27;s Meal"));
    
    // Check next week shows only next week's meal
    let next_week_start = get_week_start(next_week_day);
    let response = server.get(&format!("/meal-plan?week={}", next_week_start.format("%Y-%m-%d"))).await;
    response.assert_status_ok();
    response.assert_text_contains("Next Week&#x27;s Meal");
    let body = response.text();
    assert!(!body.contains("Today&#x27;s Meal"));
}

#[tokio::test]
async fn test_invalid_date_handling() {
    let (server, _temp_dir) = setup_test_server().await;
    
    // Test with invalid date format
    let response = server.get("/meal-plan?week=invalid-date").await;
    response.assert_status_ok(); // Should fall back to current week
}

#[tokio::test]
async fn test_meal_plan_integration_with_recipes() {
    let (server, _temp_dir) = setup_test_server_with_recipe().await;
    
    let today = Utc::now().date_naive().format("%Y-%m-%d");
    
    // Add recipe as meal
    let response = server
        .post(&format!("/meal-plan/{}/add", today))
        .form(&serde_json::json!({
            "meal_text": "Test Recipe",
            "recipe_id": "1"
        }))
        .await;
    response.assert_status_see_other();
    
    // Check meal plan shows recipe link
    let response = server.get("/meal-plan").await;
    response.assert_status_ok();
    response.assert_text_contains("Test Recipe");
    
    // Check clicking on recipe goes to recipe page
    let response = server.get("/recipes/1").await;
    response.assert_status_ok();
    response.assert_text_contains("Test Recipe");
}

/// Helper function to get the start of the week (Monday) for a given date
fn get_week_start(date: NaiveDate) -> NaiveDate {
    let days_since_monday = date.weekday().num_days_from_monday();
    date - Duration::days(days_since_monday as i64)
}

/// Helper function to set up a test server with a recipe
async fn setup_test_server_with_recipe() -> (TestServer, TempDir) {
    let (server, temp_dir) = setup_test_server().await;
    
    // Create a test recipe
    let form = MultipartForm::new()
        .add_text("title", "Test Recipe")
        .add_text("ingredients", "1 cup flour\n2 eggs")
        .add_text("instructions", "Mix and bake.");
    
    let response = server
        .post("/recipes/new")
        .multipart(form)
        .await;
    
    // Verify recipe was created successfully
    response.assert_status_see_other();
    
    (server, temp_dir)
}

/// Helper function to set up a test server with a meal
async fn setup_test_server_with_meal() -> (TestServer, TempDir) {
    let (server, temp_dir) = setup_test_server().await;
    
    let today = Utc::now().date_naive().format("%Y-%m-%d");
    let _ = server
        .post(&format!("/meal-plan/{}/add", today))
        .form(&serde_json::json!({
            "meal_text": "Test Meal",
            "recipe_id": ""
        }))
        .await;
    
    (server, temp_dir)
}