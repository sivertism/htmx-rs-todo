use axum_test::http::StatusCode;

mod common;
use common::*;

#[tokio::test]
async fn test_index_page_loads() {
    let (server, _temp_dir) = setup_test_server().await;
    
    let response = server.get("/").await;
    
    response.assert_status_ok();
    response.assert_text_contains("HTMX + Rust + SQLite = crappy todo app");
    response.assert_text_contains("Add new task");
}

#[tokio::test]
async fn test_manage_page_loads() {
    let (server, _temp_dir) = setup_test_server().await;
    
    let response = server.get("/manage").await;
    
    response.assert_status_ok();
    response.assert_text_contains("HTMX + Rust + SQLite = crappy todo app");
}

#[tokio::test]
async fn test_create_task() {
    let (server, _temp_dir) = setup_test_server().await;
    
    // First ensure we have a list to work with
    let response = server
        .post("/create_list")
        .form(&serde_json::json!({
            "name": "Test List"
        }))
        .await;
    response.assert_status_ok();
    
    // Create a task
    let response = server
        .post("/1/task")
        .form(&serde_json::json!({
            "text": "Test Task"
        }))
        .await;
    
    response.assert_status_ok();
    response.assert_text_contains("Test Task");
    response.assert_text_contains("data-id=");
}

#[tokio::test]
async fn test_task_toggle() {
    let (server, _temp_dir) = setup_test_server().await;
    
    // Create a list first
    let response = server
        .post("/create_list")
        .form(&serde_json::json!({
            "name": "Test List"
        }))
        .await;
    response.assert_status_ok();
    
    // Create a task
    let response = server
        .post("/1/task")
        .form(&serde_json::json!({
            "text": "Toggle Test Task"
        }))
        .await;
    response.assert_status_ok();
    
    // The task should have an ID, let's assume it's 1 for the first task
    // Toggle the task to completed
    let response = server.post("/task/1").await;
    
    response.assert_status_ok();
    response.assert_text_contains("Toggle Test Task");
    response.assert_text_contains("checked");
}

#[tokio::test]
async fn test_delete_task() {
    let (server, _temp_dir) = setup_test_server().await;
    
    // Create a list first
    let response = server
        .post("/create_list")
        .form(&serde_json::json!({
            "name": "Test List"
        }))
        .await;
    response.assert_status_ok();
    
    // Create a task
    let response = server
        .post("/1/task")
        .form(&serde_json::json!({
            "text": "Delete Test Task"
        }))
        .await;
    response.assert_status_ok();
    
    // Delete the task
    let response = server.delete("/task/1").await;
    
    response.assert_status_ok();
}

#[tokio::test]
async fn test_create_list() {
    let (server, _temp_dir) = setup_test_server().await;
    
    let response = server
        .post("/create_list")
        .form(&serde_json::json!({
            "name": "New Test List"
        }))
        .await;
    
    response.assert_status_ok();
    response.assert_text_contains("New Test List");
    response.assert_text_contains("option");
}

#[tokio::test]
async fn test_delete_list() {
    let (server, _temp_dir) = setup_test_server().await;
    
    // Create a list first
    let response = server
        .post("/create_list")
        .form(&serde_json::json!({
            "name": "List to Delete"
        }))
        .await;
    response.assert_status_ok();
    
    // Delete the list (assuming it gets ID 1)
    let response = server.delete("/list/1").await;
    
    response.assert_status_see_other(); // Should redirect
    response.assert_header("hx-redirect", "/manage");
}

#[tokio::test]
async fn test_reorder_tasks() {
    let (server, _temp_dir) = setup_test_server().await;
    
    // Create a list first
    let response = server
        .post("/create_list")
        .form(&serde_json::json!({
            "name": "Test List"
        }))
        .await;
    response.assert_status_ok();
    
    // Create multiple tasks
    for i in 1..=3 {
        let response = server
            .post("/1/task")
            .form(&serde_json::json!({
                "text": format!("Task {}", i)
            }))
            .await;
        response.assert_status_ok();
    }
    
    // Reorder tasks (reverse order: 3, 2, 1)
    let response = server
        .post("/reorder?list_id=1")
        .json(&serde_json::json!({
            "order": [3, 2, 1]
        }))
        .await;
    
    response.assert_status_ok();
}

#[tokio::test]
async fn test_vendor_assets_load() {
    let (server, _temp_dir) = setup_test_server().await;
    
    // Test HTMX loads
    let response = server.get("/vendor/htmx.js").await;
    response.assert_status_ok();
    response.assert_header("content-type", "application/javascript");
    response.assert_header("content-encoding", "gzip");
    
    // Test Sortable.js loads
    let response = server.get("/vendor/Sortable.js").await;
    response.assert_status_ok();
    response.assert_header("content-type", "application/javascript");
    response.assert_header("content-encoding", "gzip");
    
    // Test PicoCSS loads
    let response = server.get("/vendor/pico.min.css").await;
    response.assert_status_ok();
    response.assert_header("content-type", "text/css");
    response.assert_header("content-encoding", "gzip");
}

#[tokio::test]
async fn test_list_selection() {
    let (server, _temp_dir) = setup_test_server().await;
    
    // Create multiple lists
    for i in 1..=3 {
        let response = server
            .post("/create_list")
            .form(&serde_json::json!({
                "name": format!("List {}", i)
            }))
            .await;
        response.assert_status_ok();
    }
    
    // Test accessing specific list
    let response = server.get("/?list_id=2").await;
    response.assert_status_ok();
    response.assert_text_contains("List 2");
}

#[tokio::test]
async fn test_task_persistence() {
    let (server, _temp_dir) = setup_test_server().await;
    
    // Create a list
    let response = server
        .post("/create_list")
        .form(&serde_json::json!({
            "name": "Persistence Test"
        }))
        .await;
    response.assert_status_ok();
    
    // Create a task
    let response = server
        .post("/1/task")
        .form(&serde_json::json!({
            "text": "Persistent Task"
        }))
        .await;
    response.assert_status_ok();
    
    // Reload the page and verify task is still there
    let response = server.get("/?list_id=1").await;
    response.assert_status_ok();
    response.assert_text_contains("Persistent Task");
}