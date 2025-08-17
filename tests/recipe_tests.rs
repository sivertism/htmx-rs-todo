use axum_test::{TestServer, multipart::MultipartForm};
use tempfile::TempDir;
use axum_test::http::StatusCode;

mod common;
use common::*;

#[tokio::test]
async fn test_recipes_page_loads() {
    let (server, _temp_dir) = setup_test_server().await;
    
    let response = server.get("/recipes").await;
    
    response.assert_status_ok();
    response.assert_text_contains("Recipes");
    response.assert_text_contains("+ New Recipe");
}

#[tokio::test]
async fn test_new_recipe_form_loads() {
    let (server, _temp_dir) = setup_test_server().await;
    
    let response = server.get("/recipes/new").await;
    
    response.assert_status_ok();
    response.assert_text_contains("New Recipe");
    response.assert_text_contains("Recipe Title");
    response.assert_text_contains("Ingredients");
    response.assert_text_contains("Instructions");
}

#[tokio::test]
async fn test_create_recipe() {
    let (server, _temp_dir) = setup_test_server().await;
    
    let form = MultipartForm::new()
        .add_text("title", "Test Recipe")
        .add_text("ingredients", "1 cup flour\n2 eggs\n1 cup milk")
        .add_text("instructions", "Mix ingredients and bake at 350F for 30 minutes.");
    
    let response = server
        .post("/recipes/new")
        .multipart(form)
        .await;
    
    response.assert_status_see_other(); // Should redirect
    
    // Check that recipe appears on recipes page
    let response = server.get("/recipes").await;
    response.assert_status_ok();
    response.assert_text_contains("Test Recipe");
}

#[tokio::test]
async fn test_view_recipe() {
    let (server, _temp_dir) = setup_test_server_with_recipe().await;
    
    let response = server.get("/recipes/1").await;
    
    response.assert_status_ok();
    response.assert_text_contains("Test Recipe");
    response.assert_text_contains("1 cup flour");
    response.assert_text_contains("Mix ingredients");
    response.assert_text_contains("Edit Recipe");
    response.assert_text_contains("+ Add to List");
}

#[tokio::test]
async fn test_edit_recipe_form() {
    let (server, _temp_dir) = setup_test_server_with_recipe().await;
    
    let response = server.get("/recipes/1/edit").await;
    
    response.assert_status_ok();
    response.assert_text_contains("Edit Recipe");
    response.assert_text_contains("Test Recipe"); // Should pre-fill
}

#[tokio::test]
async fn test_update_recipe() {
    let (server, _temp_dir) = setup_test_server_with_recipe().await;
    
    let response = server
        .post("/recipes/1/edit")
        .form(&serde_json::json!({
            "title": "Updated Test Recipe",
            "ingredients": "2 cups flour\n3 eggs\n1.5 cups milk",
            "instructions": "Mix ingredients thoroughly and bake at 375F for 35 minutes."
        }))
        .await;
    
    response.assert_status_see_other(); // Should redirect
    
    // Verify the update
    let response = server.get("/recipes/1").await;
    response.assert_status_ok();
    response.assert_text_contains("Updated Test Recipe");
    response.assert_text_contains("2 cups flour");
    response.assert_text_contains("375F");
}

#[tokio::test]
async fn test_delete_recipe() {
    let (server, _temp_dir) = setup_test_server_with_recipe().await;
    
    let response = server.post("/recipes/1/delete").await;
    
    response.assert_status_ok();
    response.assert_header("HX-Redirect", "/recipes");
    
    // Verify recipe is gone
    let response = server.get("/recipes").await;
    response.assert_status_ok();
    let body = response.text();
    assert!(!body.contains("Test Recipe"));
}

#[tokio::test]
async fn test_recipe_add_to_list_form() {
    let (server, _temp_dir) = setup_test_server_with_recipe_and_list().await;
    
    let response = server.get("/recipes/1/add-to-list").await;
    
    response.assert_status_ok();
    response.assert_text_contains("Add \"Test Recipe\" to Todo List");
    response.assert_text_contains("Select Todo List");
    response.assert_text_contains("Test List");
    response.assert_text_contains("1 cup flour");
    response.assert_text_contains("Select All / Deselect All");
}

#[tokio::test]
async fn test_add_recipe_to_todo_list() {
    let (server, _temp_dir) = setup_test_server_with_recipe_and_list().await;
    
    // Test with a single ingredient first (Vec with one element)
    let response = server
        .post("/recipes/1/add-to-list")
        .form(&[
            ("list_id", "1"),
            ("ingredients", "1 cup flour"),
        ])
        .await;
    
    // Accept either redirect or ok status for now
    assert!(
        response.status_code() == 303 || response.status_code() == 200,
        "Expected 303 or 200, got {}", response.status_code()
    );
    
    // Check that ingredient was added to the todo list
    let response = server.get("/?list_id=1").await;
    response.assert_status_ok();
    response.assert_text_contains("1 cup flour");
}

#[tokio::test]
async fn test_empty_recipes_page() {
    let (server, _temp_dir) = setup_test_server().await;
    
    let response = server.get("/recipes").await;
    
    response.assert_status_ok();
    response.assert_text_contains("No recipes yet");
    response.assert_text_contains("Create your first recipe");
}

#[tokio::test]
async fn test_recipe_validation() {
    let (server, _temp_dir) = setup_test_server().await;
    
    // Try to create recipe without title
    let form = MultipartForm::new()
        .add_text("title", "")
        .add_text("ingredients", "Some ingredients")
        .add_text("instructions", "Some instructions");
    
    let response = server
        .post("/recipes/new")
        .multipart(form)
        .await;
    
    response.assert_status_bad_request();
}

#[tokio::test]
async fn test_recipe_not_found() {
    let (server, _temp_dir) = setup_test_server().await;
    
    let response = server.get("/recipes/999").await;
    
    response.assert_status_not_found();
}

/// Helper function to set up a test server with a recipe
async fn setup_test_server_with_recipe() -> (TestServer, TempDir) {
    let (server, temp_dir) = setup_test_server().await;
    
    // Create a test recipe
    let form = MultipartForm::new()
        .add_text("title", "Test Recipe")
        .add_text("ingredients", "1 cup flour\n2 eggs\n1 cup milk")
        .add_text("instructions", "Mix ingredients and bake at 350F for 30 minutes.");
    
    let _ = server
        .post("/recipes/new")
        .multipart(form)
        .await;
    
    (server, temp_dir)
}

/// Helper function to set up a test server with a recipe and a todo list
async fn setup_test_server_with_recipe_and_list() -> (TestServer, TempDir) {
    let (server, temp_dir) = setup_test_server().await;
    
    // Create a test list
    let _ = server
        .post("/create_list")
        .form(&serde_json::json!({
            "name": "Test List"
        }))
        .await;
    
    // Create a test recipe
    let form = MultipartForm::new()
        .add_text("title", "Test Recipe")
        .add_text("ingredients", "1 cup flour\n2 eggs\n1 cup milk")
        .add_text("instructions", "Mix ingredients and bake at 350F for 30 minutes.");
    
    let _ = server
        .post("/recipes/new")
        .multipart(form)
        .await;
    
    (server, temp_dir)
}