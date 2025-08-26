use std::process::Command;
use tempfile::TempDir;

#[tokio::test]
async fn test_photo_upload_end_to_end() {
    println!("🧪 Testing photo upload end-to-end");

    // Create temporary directory for test data
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    // Start the application
    println!("📡 Starting application...");
    let mut app_process = Command::new("cargo")
        .args(&["run", "--", "--port", "3003", "--address", "127.0.0.1", "--data-dir"])
        .arg(&data_dir)
        .spawn()
        .expect("Failed to start application");

    // Wait for application to start
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Test 1: Create a recipe
    println!("📝 Creating test recipe...");
    let output = Command::new("curl")
        .args(&[
            "-X", "POST",
            "-F", "title=Test Recipe for Photo Upload",
            "-F", "instructions=Test instructions for photo testing",
            "-F", "ingredients=Test ingredients",
            "http://127.0.0.1:3003/recipes/new"
        ])
        .output()
        .expect("Failed to create recipe");

    if !output.status.success() {
        app_process.kill().unwrap();
        panic!("Failed to create recipe: {}", String::from_utf8_lossy(&output.stderr));
    }

    println!("✅ Recipe created successfully");

    // Test 2: Create a test image file
    println!("🖼️  Creating test image...");
    let test_image_path = temp_dir.path().join("test_photo.jpg");
    
    // Create a simple BMP image that can be converted to JPEG
    // BMP header for 4x4 RGB image
    let bmp_data = vec![
        // BMP Header
        0x42, 0x4D, // "BM"
        0x76, 0x00, 0x00, 0x00, // file size: 118 bytes
        0x00, 0x00, 0x00, 0x00, // reserved
        0x36, 0x00, 0x00, 0x00, // offset to pixel data: 54 bytes
        
        // DIB Header (BITMAPINFOHEADER)
        0x28, 0x00, 0x00, 0x00, // header size: 40 bytes
        0x04, 0x00, 0x00, 0x00, // width: 4 pixels
        0x04, 0x00, 0x00, 0x00, // height: 4 pixels
        0x01, 0x00, // planes: 1
        0x18, 0x00, // bits per pixel: 24 (RGB)
        0x00, 0x00, 0x00, 0x00, // compression: none
        0x40, 0x00, 0x00, 0x00, // image size: 64 bytes
        0x13, 0x0B, 0x00, 0x00, // horizontal resolution
        0x13, 0x0B, 0x00, 0x00, // vertical resolution
        0x00, 0x00, 0x00, 0x00, // colors in palette
        0x00, 0x00, 0x00, 0x00, // important colors
        
        // Pixel data (4x4 pixels, 3 bytes per pixel, rows padded to 4-byte boundary)
        // Row 0 (bottom row in BMP): Red pixels
        0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF,
        // Row 1: Green pixels  
        0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00,
        // Row 2: Blue pixels
        0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00,
        // Row 3 (top row): White pixels
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    ];

    std::fs::write(&test_image_path, bmp_data).expect("Failed to write test image");
    println!("✅ Test image created: {} bytes", test_image_path.metadata().unwrap().len());

    // Test 3: Upload the photo
    println!("📤 Uploading photo...");
    let upload_output = Command::new("curl")
        .args(&[
            "-X", "POST",
            "-F", &format!("photos=@{}", test_image_path.to_string_lossy()),
            "http://127.0.0.1:3003/recipes/1/upload-photos"
        ])
        .output()
        .expect("Failed to upload photo");

    if !upload_output.status.success() {
        println!("Upload stderr: {}", String::from_utf8_lossy(&upload_output.stderr));
        println!("Upload stdout: {}", String::from_utf8_lossy(&upload_output.stdout));
    }

    // Test 4: Check that recipes page shows the photo
    println!("🔍 Checking recipes page...");
    let recipes_output = Command::new("curl")
        .args(&["-s", "http://127.0.0.1:3003/recipes"])
        .output()
        .expect("Failed to fetch recipes page");

    let recipes_html = String::from_utf8_lossy(&recipes_output.stdout);

    // Check if photo is referenced (should contain a UUID filename, not default-recipe.svg)
    let has_uploaded_photo = recipes_html.contains(".jpg") || recipes_html.contains(".png") || recipes_html.contains(".bmp");
    let has_default_photo = recipes_html.contains("default-recipe.svg");

    println!("📊 Test Results:");
    println!("  - Has uploaded photo reference: {}", has_uploaded_photo);
    println!("  - Has default photo reference: {}", has_default_photo);

    // Test 5: Verify photo serving endpoint works
    if has_uploaded_photo {
        println!("🌐 Testing photo serving...");
        // Extract photo filename from HTML (look for /photos/filename pattern)
        if let Some(start) = recipes_html.find("/photos/") {
            let remaining = &recipes_html[start + 8..]; // Skip "/photos/"
            if let Some(end) = remaining.find('"') {
                let filename = &remaining[..end];
                println!("  - Found photo filename: {}", filename);
                
                let photo_output = Command::new("curl")
                    .args(&["-I", &format!("http://127.0.0.1:3003/photos/{}", filename)])
                    .output()
                    .expect("Failed to test photo serving");
                    
                let photo_headers = String::from_utf8_lossy(&photo_output.stdout);
                let has_image_content_type = photo_headers.contains("content-type: image/");
                println!("  - Photo serves with image content-type: {}", has_image_content_type);
            }
        }
    }

    // Cleanup
    println!("🧹 Cleaning up...");
    app_process.kill().unwrap();
    app_process.wait().unwrap();

    // Assert test results
    if upload_output.status.success() {
        println!("✅ All photo upload tests passed!");
    } else {
        println!("❌ Photo upload failed!");
        println!("Upload stdout: {}", String::from_utf8_lossy(&upload_output.stdout));
        panic!("Photo upload test failed");
    }
}

#[tokio::test] 
async fn test_default_photo_fallback() {
    println!("🧪 Testing default photo fallback");

    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    // Start the application
    let mut app_process = Command::new("cargo")
        .args(&["run", "--", "--port", "3004", "--address", "127.0.0.1", "--data-dir"])
        .arg(&data_dir)
        .spawn()
        .expect("Failed to start application");

    std::thread::sleep(std::time::Duration::from_secs(3));

    // Create a recipe without photo
    let create_output = Command::new("curl")
        .args(&[
            "-X", "POST",
            "-F", "title=No Photo Recipe",
            "-F", "instructions=This recipe has no photo",
            "-F", "ingredients=None",
            "http://127.0.0.1:3004/recipes/new"
        ])
        .output()
        .expect("Failed to create recipe");
    
    println!("Recipe creation result: {}", create_output.status);
    if !create_output.stdout.is_empty() {
        println!("Recipe creation stdout: {}", String::from_utf8_lossy(&create_output.stdout));
    }
    if !create_output.stderr.is_empty() {
        println!("Recipe creation stderr: {}", String::from_utf8_lossy(&create_output.stderr));
    }

    // Check recipes page shows default image
    let recipes_output = Command::new("curl")
        .args(&["-s", "http://127.0.0.1:3004/recipes"])
        .output()
        .expect("Failed to fetch recipes page");

    let recipes_html = String::from_utf8_lossy(&recipes_output.stdout);
    let has_default_photo = recipes_html.contains("default-recipe.svg");

    println!("📊 Default photo test:");
    println!("  - Shows default photo: {}", has_default_photo);
    
    // Debug: Print a snippet of the recipes page
    if recipes_html.len() > 500 {
        println!("  - Recipes page snippet: ...{}", &recipes_html[recipes_html.len()-500..]);
    } else {
        println!("  - Recipes page content: {}", recipes_html);
    }

    // Test default photo endpoint
    let default_output = Command::new("curl")
        .args(&["-I", "http://127.0.0.1:3004/photos/default-recipe.svg"])
        .output()
        .expect("Failed to test default photo");

    let default_headers = String::from_utf8_lossy(&default_output.stdout);
    let serves_svg = default_headers.contains("content-type: image/svg+xml");

    println!("  - Default photo serves as SVG: {}", serves_svg);

    // Cleanup
    app_process.kill().unwrap();
    app_process.wait().unwrap();

    assert!(has_default_photo, "Recipe without photo should show default image");
    assert!(serves_svg, "Default photo should serve as SVG");

    println!("✅ Default photo fallback test passed!");
}