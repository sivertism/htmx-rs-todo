use axum_test::TestServer;
use tempfile::TempDir;

// Import from the library
use htmx_rs_todo::{database::Database, AppState, create_app};

/// Sets up a test server with a temporary database
pub async fn setup_test_server() -> (TestServer, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test.db");
    
    let db = Database::new(db_path)
        .await
        .expect("Failed to create test database");
    
    let photos_dir = temp_dir.path().join("photos");
    std::fs::create_dir_all(&photos_dir).expect("Failed to create photos directory");
    let app_state = AppState { db, photos_dir };
    
    let app = create_app(app_state);
    let server = TestServer::new(app).expect("Failed to create test server");
    
    (server, temp_dir)
}

/// Sets up a test server with some pre-populated data
pub async fn setup_test_server_with_data() -> (TestServer, TempDir) {
    let (server, temp_dir) = setup_test_server().await;
    
    // Create a default list
    let _ = server
        .post("/create_list")
        .form(&serde_json::json!({
            "name": "Default List"
        }))
        .await;
    
    // Create some test tasks
    for i in 1..=3 {
        let _ = server
            .post("/1/task")
            .form(&serde_json::json!({
                "text": format!("Test Task {}", i)
            }))
            .await;
    }
    
    (server, temp_dir)
}