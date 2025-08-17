use htmx_rs_todo::*;
use tempfile::TempDir;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::util::ServiceExt;

#[tokio::test]
async fn test_photo_upload_integration() {
    // Create temporary directory for test data
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();
    let photos_dir = data_dir.join("photos");
    std::fs::create_dir_all(&photos_dir).unwrap();

    // Initialize database
    let db = database::Database::new(data_dir.join("test.db")).await.unwrap();
    let state = AppState { db, photos_dir };

    // Create the app
    let app = create_app(state.clone());

    // Step 1: Create a test recipe
    println!("Creating test recipe...");
    let create_recipe_body = "title=Test%20Recipe&instructions=Test%20instructions&ingredients=Test%20ingredients";
    let request = Request::builder()
        .method("POST")
        .uri("/recipes/new")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(create_recipe_body))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    
    // Get the recipe ID from database (should be 1 for first recipe)
    let recipes = state.db.get_recipes().await.unwrap();
    assert!(!recipes.is_empty(), "Recipe should have been created");
    let recipe_id = recipes[0].id;
    println!("Created recipe with ID: {}", recipe_id);

    // Step 2: Create a simple test image (PNG)
    let test_image_data = create_test_png();
    println!("Created test image: {} bytes", test_image_data.len());

    // Step 3: Create multipart form data for photo upload
    let boundary = "----test-boundary-12345";
    let multipart_body = format!(
        "--{boundary}\r\n\
        Content-Disposition: form-data; name=\"photos\"; filename=\"test.png\"\r\n\
        Content-Type: image/png\r\n\
        \r\n\
        {image_data}\r\n\
        --{boundary}--\r\n",
        boundary = boundary,
        image_data = String::from_utf8_lossy(&test_image_data)
    );

    // Step 4: Upload the photo
    println!("Uploading photo...");
    let upload_request = Request::builder()
        .method("POST")
        .uri(format!("/recipes/{}/upload-photos", recipe_id))
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .body(Body::from(multipart_body))
        .unwrap();

    let upload_response = app.clone().oneshot(upload_request).await.unwrap();
    let upload_status = upload_response.status();
    println!("Upload response status: {}", upload_status);
    
    if upload_status != StatusCode::SEE_OTHER {
        let body_bytes = axum::body::to_bytes(upload_response.into_body(), usize::MAX).await.unwrap();
        let body_text = String::from_utf8_lossy(&body_bytes);
        println!("Upload response body: {}", body_text);
        panic!("Upload failed with status: {}", upload_status);
    }

    // Step 5: Verify photo was saved to database
    println!("Checking database for uploaded photo...");
    let photos = state.db.get_recipe_photos(recipe_id).await.unwrap();
    assert!(!photos.is_empty(), "Photo should have been saved to database");
    assert_eq!(photos.len(), 1, "Should have exactly one photo");
    
    let photo = &photos[0];
    println!("Photo saved: filename={}, size={}", photo.filename, photo.file_size);
    assert_eq!(photo.original_name, "test.png");
    assert_eq!(photo.mime_type, "image/png");
    assert!(photo.thumbnail_blob.is_some(), "Thumbnail should have been generated");

    // Step 6: Verify photo file exists on disk
    let photo_path = state.photos_dir.join(&photo.filename);
    assert!(photo_path.exists(), "Photo file should exist on disk at {:?}", photo_path);
    let file_size = std::fs::metadata(&photo_path).unwrap().len();
    assert_eq!(file_size, photo.file_size as u64, "File size should match database");

    // Step 7: Test photo serving endpoint
    println!("Testing photo serving endpoint...");
    let photo_request = Request::builder()
        .method("GET")
        .uri(format!("/photos/{}", photo.filename))
        .body(Body::empty())
        .unwrap();

    let photo_response = app.clone().oneshot(photo_request).await.unwrap();
    assert_eq!(photo_response.status(), StatusCode::OK);
    
    let headers = photo_response.headers();
    assert_eq!(headers.get("content-type").unwrap(), "image/png");

    // Step 8: Test recipes page shows thumbnail
    println!("Testing recipes page shows thumbnail...");
    let recipes_request = Request::builder()
        .method("GET")
        .uri("/recipes")
        .body(Body::empty())
        .unwrap();

    let recipes_response = app.clone().oneshot(recipes_request).await.unwrap();
    assert_eq!(recipes_response.status(), StatusCode::OK);
    
    let body_bytes = axum::body::to_bytes(recipes_response.into_body(), usize::MAX).await.unwrap();
    let body_text = String::from_utf8_lossy(&body_bytes);
    
    // Check that the recipe shows the photo instead of default
    assert!(body_text.contains(&format!("/photos/{}", photo.filename)), 
            "Recipes page should contain link to uploaded photo");
    assert!(!body_text.contains("default-recipe.svg"), 
            "Recipes page should not show default image when photo exists");

    // Step 9: Test thumbnail endpoint
    println!("Testing thumbnail endpoint...");
    let thumbnail_request = Request::builder()
        .method("GET")
        .uri(format!("/thumbnails/{}", photo.id))
        .body(Body::empty())
        .unwrap();

    let thumbnail_response = app.clone().oneshot(thumbnail_request).await.unwrap();
    assert_eq!(thumbnail_response.status(), StatusCode::OK);
    
    let thumb_headers = thumbnail_response.headers();
    assert_eq!(thumb_headers.get("content-type").unwrap(), "image/jpeg");

    println!("✅ All photo upload tests passed!");
}

fn create_test_png() -> Vec<u8> {
    // Create a minimal valid PNG (1x1 pixel, red)
    // PNG signature + IHDR + IDAT + IEND chunks
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR length
        0x49, 0x48, 0x44, 0x52, // IHDR
        0x00, 0x00, 0x00, 0x01, // width = 1
        0x00, 0x00, 0x00, 0x01, // height = 1
        0x08, 0x02, 0x00, 0x00, 0x00, // bit depth=8, color type=2 (RGB), compression=0, filter=0, interlace=0
        0x90, 0x77, 0x53, 0xDE, // IHDR CRC
        0x00, 0x00, 0x00, 0x0C, // IDAT length
        0x49, 0x44, 0x41, 0x54, // IDAT
        0x78, 0x9C, 0x62, 0xF8, 0x0F, 0x00, 0x01, 0x01, 0x01, 0x00, 0x18, 0xDD, // compressed RGB pixel data
        0x8D, 0xB4, 0x1C, // IDAT CRC
        0x00, 0x00, 0x00, 0x00, // IEND length
        0x49, 0x45, 0x4E, 0x44, // IEND
        0xAE, 0x42, 0x60, 0x82, // IEND CRC
    ]
}

#[tokio::test]
async fn test_default_photo_fallback() {
    // Test that recipes without photos show default image
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();
    let photos_dir = data_dir.join("photos");
    std::fs::create_dir_all(&photos_dir).unwrap();

    let db = database::Database::new(data_dir.join("test.db")).await.unwrap();
    let state = AppState { db, photos_dir };
    let app = create_app(state.clone());

    // Create recipe without photo
    let create_recipe_body = "title=No%20Photo%20Recipe&instructions=Test&ingredients=Test";
    let request = Request::builder()
        .method("POST")
        .uri("/recipes/new")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(create_recipe_body))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    // Test recipes page shows default image
    let recipes_request = Request::builder()
        .method("GET")
        .uri("/recipes")
        .body(Body::empty())
        .unwrap();

    let recipes_response = app.clone().oneshot(recipes_request).await.unwrap();
    assert_eq!(recipes_response.status(), StatusCode::OK);
    
    let body_bytes = axum::body::to_bytes(recipes_response.into_body(), usize::MAX).await.unwrap();
    let body_text = String::from_utf8_lossy(&body_bytes);
    
    assert!(body_text.contains("default-recipe.svg"), 
            "Recipes page should show default image when no photo exists");

    // Test default photo endpoint
    let default_request = Request::builder()
        .method("GET")
        .uri("/photos/default-recipe.svg")
        .body(Body::empty())
        .unwrap();

    let default_response = app.oneshot(default_request).await.unwrap();
    assert_eq!(default_response.status(), StatusCode::OK);
    
    let headers = default_response.headers();
    assert_eq!(headers.get("content-type").unwrap(), "image/svg+xml");

    println!("✅ Default photo fallback test passed!");
}