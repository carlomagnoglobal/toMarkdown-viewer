//! End-to-end integration tests covering the complete workflow:
//! File detection → Viewer instantiation → File operations → Vault operations
//!
//! This test suite creates a complete test vault with sample files and verifies
//! that all subsystems work together correctly.

use std::fs;
use tempfile::TempDir;

// Import library modules
use to_markdown_gui::commands::file_ops;
use to_markdown_gui::file_types::{detect_file_type, FileType};
use to_markdown_gui::vault;
use to_markdown_gui::viewers::{CodeViewer, FileViewer, HexViewer, ImageViewer, MarkdownViewer};

// ============================================================================
// TEST VAULT SETUP
// ============================================================================

/// Create a comprehensive test vault with sample files of all types
fn create_test_vault() -> Result<TempDir, Box<dyn std::error::Error>> {
    let vault = TempDir::new()?;
    let vault_path = vault.path();

    // Create a documents folder
    fs::create_dir(vault_path.join("documents"))?;
    fs::create_dir(vault_path.join("code"))?;
    fs::create_dir(vault_path.join("images"))?;
    fs::create_dir(vault_path.join("assets"))?;

    // Create markdown files
    fs::write(
        vault_path.join("README.md"),
        "# Test Vault\n\nThis is a test markdown file.\n\n## Features\n- File detection\n- Viewer support\n- File operations",
    )?;
    fs::write(
        vault_path.join("documents/notes.md"),
        "# Notes\n\n## Note 1\nSome content here.\n",
    )?;

    // Create code files in various languages
    fs::write(
        vault_path.join("code/hello.rs"),
        "fn main() {\n    println!(\"Hello, world!\");\n}\n",
    )?;
    fs::write(
        vault_path.join("code/script.py"),
        "#!/usr/bin/env python3\nprint('Hello, world!')\n",
    )?;
    fs::write(
        vault_path.join("code/index.js"),
        "console.log('Hello, world!');\n",
    )?;
    fs::write(
        vault_path.join("code/style.css"),
        "body { color: #333; font-family: sans-serif; }\n",
    )?;
    fs::write(
        vault_path.join("code/config.json"),
        r#"{ "name": "test-project", "version": "1.0.0" }"#,
    )?;
    fs::write(
        vault_path.join("code/data.yaml"),
        "key: value\nlist:\n  - item1\n  - item2\n",
    )?;
    fs::write(
        vault_path.join("code/example.html"),
        "<html><head><title>Example</title></head><body><p>Hello</p></body></html>\n",
    )?;

    // Create image files (simple binary data)
    // PNG: minimal valid PNG file structure
    fs::write(
        vault_path.join("images/test.png"),
        &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, // IHDR chunk length
            0x49, 0x48, 0x44, 0x52, // IHDR
            0x00, 0x00, 0x00, 0x01, // Width: 1
            0x00, 0x00, 0x00, 0x01, // Height: 1
            0x08, 0x02, 0x00, 0x00, 0x00, // Bit depth, color type, etc.
            0x90, 0x77, 0x53, 0xDE, // CRC
            0x00, 0x00, 0x00, 0x0C, // IDAT chunk length
            0x49, 0x44, 0x41, 0x54, // IDAT
            0x08, 0x99, 0x01, 0x01, 0x00, 0x00, 0xFE, 0xFF, 0x00, 0x00, 0x00, 0x02, // Data
            0x00, 0x01, 0xE5, 0x27, 0xDE, 0xFC, // CRC
        ],
    )?;

    // JPEG: minimal valid JPEG structure
    fs::write(
        vault_path.join("images/test.jpg"),
        &[
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x00, // JPEG header
            0xFF, 0xD9, // EOI marker
        ],
    )?;

    // GIF: minimal valid GIF structure
    fs::write(
        vault_path.join("images/test.gif"),
        &[
            0x47, 0x49, 0x46, 0x38, 0x39, 0x61, // GIF89a
            0x01, 0x00, 0x01, 0x00, 0x80, 0x00, 0x00, // Logical screen descriptor
            0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, // Global color table + EOF
        ],
    )?;

    // WebP: minimal valid WebP structure
    fs::write(
        vault_path.join("images/test.webp"),
        &[
            0x52, 0x49, 0x46, 0x46, // RIFF
            0x24, 0x00, 0x00, 0x00, // File size
            0x57, 0x45, 0x42, 0x50, // WEBP
            0x56, 0x50, 0x38, 0x4C, // VP8L
            0x18, 0x00, 0x00, 0x00, // VP8L chunk size
            0x2F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
    )?;

    // Create binary/unknown files
    fs::write(vault_path.join("assets/data.bin"), &[0x00, 0x01, 0x02, 0x03, 0xFF])?;

    // Create a plain text file
    fs::write(
        vault_path.join("documents/plaintext.txt"),
        "This is a plain text file.\nIt contains multiple lines.\n",
    )?;

    Ok(vault)
}

// ============================================================================
// FILE TYPE DETECTION TESTS
// ============================================================================

#[test]
fn test_integration_file_type_detection_markdown() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let readme = vault.path().join("README.md");

    let file_type = detect_file_type(&readme);
    assert_eq!(file_type, FileType::Markdown);
}

#[test]
fn test_integration_file_type_detection_code_files() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let code_dir = vault.path().join("code");

    // Test Rust
    let rust_file = code_dir.join("hello.rs");
    match detect_file_type(&rust_file) {
        FileType::Code { language } => assert_eq!(language, "rust"),
        _ => panic!("Expected Rust code type"),
    }

    // Test Python
    let py_file = code_dir.join("script.py");
    match detect_file_type(&py_file) {
        FileType::Code { language } => assert_eq!(language, "python"),
        _ => panic!("Expected Python code type"),
    }

    // Test JavaScript
    let js_file = code_dir.join("index.js");
    match detect_file_type(&js_file) {
        FileType::Code { language } => assert_eq!(language, "javascript"),
        _ => panic!("Expected JavaScript code type"),
    }

    // Test CSS
    let css_file = code_dir.join("style.css");
    match detect_file_type(&css_file) {
        FileType::Code { language } => assert_eq!(language, "css"),
        _ => panic!("Expected CSS code type"),
    }

    // Test JSON
    let json_file = code_dir.join("config.json");
    match detect_file_type(&json_file) {
        FileType::Code { language } => assert_eq!(language, "json"),
        _ => panic!("Expected JSON code type"),
    }

    // Test YAML
    let yaml_file = code_dir.join("data.yaml");
    match detect_file_type(&yaml_file) {
        FileType::Code { language } => assert_eq!(language, "yaml"),
        _ => panic!("Expected YAML code type"),
    }

    // Test HTML
    let html_file = code_dir.join("example.html");
    match detect_file_type(&html_file) {
        FileType::Code { language } => assert_eq!(language, "html"),
        _ => panic!("Expected HTML code type"),
    }
}

#[test]
fn test_integration_file_type_detection_images() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let images_dir = vault.path().join("images");

    // Test PNG
    let png_file = images_dir.join("test.png");
    match detect_file_type(&png_file) {
        FileType::Image { format } => assert_eq!(format, "png"),
        _ => panic!("Expected PNG image type"),
    }

    // Test JPEG
    let jpg_file = images_dir.join("test.jpg");
    match detect_file_type(&jpg_file) {
        FileType::Image { format } => assert_eq!(format, "jpeg"),
        _ => panic!("Expected JPEG image type"),
    }

    // Test GIF
    let gif_file = images_dir.join("test.gif");
    match detect_file_type(&gif_file) {
        FileType::Image { format } => assert_eq!(format, "gif"),
        _ => panic!("Expected GIF image type"),
    }

    // Test WebP
    let webp_file = images_dir.join("test.webp");
    match detect_file_type(&webp_file) {
        FileType::Image { format } => assert_eq!(format, "webp"),
        _ => panic!("Expected WebP image type"),
    }
}

#[test]
fn test_integration_file_type_detection_binary() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let binary_file = vault.path().join("assets/data.bin");

    let file_type = detect_file_type(&binary_file);
    assert_eq!(file_type, FileType::Hex);
}

#[test]
fn test_integration_file_type_detection_text() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let text_file = vault.path().join("documents/plaintext.txt");

    match detect_file_type(&text_file) {
        FileType::Code { language } => assert_eq!(language, "plaintext"),
        _ => panic!("Expected plaintext code type"),
    }
}

// ============================================================================
// VIEWER INSTANTIATION TESTS
// ============================================================================

#[test]
fn test_integration_markdown_viewer_instantiation() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let readme = vault.path().join("README.md");

    let content = fs::read_to_string(&readme).expect("Failed to read markdown file");
    let file_size = fs::metadata(&readme).expect("Failed to get metadata").len();

    // Create a markdown viewer
    let viewer = MarkdownViewer::new(readme.clone(), content, file_size);

    // Verify viewer state
    let state = viewer.get_state();
    assert_eq!(state.file_type, "markdown");
    assert_eq!(state.file_path, readme);
    assert!(!state.modified);
    assert!(state.file_size_bytes > 0);

    // Verify render works
    let rendered = viewer.render().expect("Viewer render should succeed");
    assert!(!rendered.is_empty());
}

#[test]
fn test_integration_code_viewer_instantiation() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let rust_file = vault.path().join("code/hello.rs");

    let content = fs::read_to_string(&rust_file).expect("Failed to read code file");

    // Create a code viewer
    let viewer = CodeViewer::new(rust_file.clone(), "rust".to_string(), content, false)
        .expect("CodeViewer creation should succeed");

    // Verify viewer state
    let state = viewer.get_state();
    assert_eq!(state.file_type, "code");
    assert_eq!(state.file_path, rust_file);

    // Verify render works
    let rendered = viewer.render().expect("Viewer render should succeed");
    assert!(!rendered.is_empty());
}

#[test]
fn test_integration_hex_viewer_instantiation() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let binary_file = vault.path().join("assets/data.bin");

    let file_size = fs::metadata(&binary_file).expect("Failed to get metadata").len();
    let bytes = fs::read(&binary_file).expect("Failed to read binary file");

    // Create a hex viewer
    let viewer = HexViewer::new_from_bytes(binary_file.clone(), bytes, file_size)
        .expect("HexViewer creation should succeed");

    // Verify viewer state
    let state = viewer.get_state();
    assert_eq!(state.file_type, "hex");
    assert_eq!(state.file_path, binary_file);

    // Verify render works
    let rendered = viewer.render().expect("Viewer render should succeed");
    assert!(!rendered.is_empty());
}

#[test]
fn test_integration_image_viewer_instantiation() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let image_file = vault.path().join("images/test.png");

    // Create an image viewer with dummy dimensions (1x1)
    let viewer = ImageViewer::new(image_file.clone(), "png".to_string(), 1, 1)
        .expect("ImageViewer creation should succeed");

    // Verify viewer state
    let state = viewer.get_state();
    assert_eq!(state.file_type, "image");
    assert_eq!(state.file_path, image_file);

    // Verify render works
    let rendered = viewer.render().expect("Viewer render should succeed");
    assert!(!rendered.is_empty());
}

// ============================================================================
// FILE OPERATIONS TESTS
// ============================================================================

#[test]
fn test_integration_file_duplicate_and_verify() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let original_file = vault.path().join("README.md");
    let original_content = fs::read_to_string(&original_file).expect("Failed to read original");

    // Duplicate the file
    let duplicate_path = file_ops::duplicate_file_impl(&original_file)
        .expect("Duplicate should succeed");

    // Verify duplicate exists and has correct content
    assert!(duplicate_path.exists());
    let dup_content = fs::read_to_string(&duplicate_path).expect("Failed to read duplicate");
    assert_eq!(dup_content, original_content);

    // Verify the naming
    assert!(duplicate_path.file_name().unwrap().to_str().unwrap().contains("copy"));
}

#[test]
fn test_integration_file_duplicate_and_duplicate_again() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let original_file = vault.path().join("README.md");

    // First duplicate
    let dup1 = file_ops::duplicate_file_impl(&original_file)
        .expect("First duplicate should succeed");
    assert!(dup1.exists());

    // Second duplicate
    let dup2 = file_ops::duplicate_file_impl(&original_file)
        .expect("Second duplicate should succeed");
    assert!(dup2.exists());
    assert_ne!(dup1, dup2);

    // Verify both have original content
    let original_content = fs::read_to_string(&original_file).expect("Failed to read original");
    let dup1_content = fs::read_to_string(&dup1).expect("Failed to read dup1");
    let dup2_content = fs::read_to_string(&dup2).expect("Failed to read dup2");
    assert_eq!(dup1_content, original_content);
    assert_eq!(dup2_content, original_content);
}

#[test]
fn test_integration_create_markdown_note() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let documents_dir = vault.path().join("documents");

    // Create a new note
    let note_path = file_ops::create_markdown_note_impl(&documents_dir, "new_note.md")
        .expect("Create note should succeed");

    // Verify the note exists
    assert!(note_path.exists());
    assert_eq!(note_path.file_name().unwrap(), "new_note.md");

    // Verify it's empty
    let content = fs::read_to_string(&note_path).expect("Failed to read note");
    assert_eq!(content, "");

    // Verify it's in the right folder
    assert_eq!(note_path.parent().unwrap(), documents_dir);
}

#[test]
fn test_integration_create_multiple_notes() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let documents_dir = vault.path().join("documents");

    // Create multiple notes
    let note1 = file_ops::create_markdown_note_impl(&documents_dir, "note_1.md")
        .expect("Note 1 creation should succeed");
    let note2 = file_ops::create_markdown_note_impl(&documents_dir, "note_2.md")
        .expect("Note 2 creation should succeed");
    let note3 = file_ops::create_markdown_note_impl(&documents_dir, "note_3.md")
        .expect("Note 3 creation should succeed");

    // Verify all exist
    assert!(note1.exists());
    assert!(note2.exists());
    assert!(note3.exists());

    // Verify they are distinct
    assert_ne!(note1, note2);
    assert_ne!(note2, note3);
}

// ============================================================================
// VAULT OPERATIONS TESTS
// ============================================================================

#[test]
fn test_integration_vault_initialization() {
    let vault = TempDir::new().expect("Failed to create temp vault");
    let vault_path = vault.path();

    // Initialize vault database
    let vault_db = vault::init_vault_db(vault_path)
        .expect("Vault initialization should succeed");

    // Verify .tomarkdown directory was created
    let tomarkdown_dir = vault_path.join(".tomarkdown");
    assert!(tomarkdown_dir.exists());
    assert!(tomarkdown_dir.is_dir());

    // Verify database file exists
    let db_file = tomarkdown_dir.join("vault.db");
    assert!(db_file.exists());
    assert!(db_file.is_file());

    // Verify vault root is correct
    assert_eq!(vault_db.vault_root, vault_path);
}

#[test]
fn test_integration_vault_database_ready_for_queries() {
    let vault = TempDir::new().expect("Failed to create temp vault");
    let vault_path = vault.path();

    // Initialize vault database
    let vault_db = vault::init_vault_db(vault_path)
        .expect("Vault initialization should succeed");

    // Verify connection is valid by attempting a query
    // We use a simple query that doesn't depend on schema initialization
    let result = vault_db.conn.prepare("SELECT 1");
    assert!(result.is_ok(), "Should be able to prepare statements");
}

// ============================================================================
// END-TO-END WORKFLOW TESTS
// ============================================================================

#[test]
fn test_integration_detect_open_and_render_markdown() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let file_path = vault.path().join("README.md");

    // Step 1: Detect file type
    let file_type = detect_file_type(&file_path);
    assert_eq!(file_type, FileType::Markdown);

    // Step 2: Read content
    let content = fs::read_to_string(&file_path).expect("Failed to read file");
    let file_size = fs::metadata(&file_path).expect("Failed to get metadata").len();

    // Step 3: Create appropriate viewer
    let viewer = MarkdownViewer::new(file_path.clone(), content, file_size);

    // Step 4: Get state and render
    let state = viewer.get_state();
    assert_eq!(state.file_type, "markdown");
    let rendered = viewer.render().expect("Render should succeed");
    assert!(!rendered.is_empty());
}

#[test]
fn test_integration_detect_open_and_render_code() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let file_path = vault.path().join("code/hello.rs");

    // Step 1: Detect file type
    let file_type = detect_file_type(&file_path);
    assert!(matches!(file_type, FileType::Code { .. }));

    // Step 2: Read content
    let content = fs::read_to_string(&file_path).expect("Failed to read file");

    // Step 3: Create appropriate viewer
    if let FileType::Code { language } = file_type {
        let viewer = CodeViewer::new(file_path.clone(), language, content, false)
            .expect("CodeViewer creation should succeed");

        // Step 4: Get state and render
        let state = viewer.get_state();
        assert_eq!(state.file_type, "code");
        let rendered = viewer.render().expect("Render should succeed");
        assert!(!rendered.is_empty());
    } else {
        panic!("Expected Code file type");
    }
}

#[test]
fn test_integration_detect_open_and_render_image() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let file_path = vault.path().join("images/test.png");

    // Step 1: Detect file type
    let file_type = detect_file_type(&file_path);
    assert!(matches!(file_type, FileType::Image { .. }));

    // Step 3: Create appropriate viewer
    if let FileType::Image { format } = file_type {
        let viewer = ImageViewer::new(file_path.clone(), format, 1, 1)
            .expect("ImageViewer creation should succeed");

        // Step 4: Get state and render
        let state = viewer.get_state();
        assert_eq!(state.file_type, "image");
        let rendered = viewer.render().expect("Render should succeed");
        assert!(!rendered.is_empty());
    } else {
        panic!("Expected Image file type");
    }
}

#[test]
fn test_integration_detect_open_and_render_binary() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let file_path = vault.path().join("assets/data.bin");

    // Step 1: Detect file type
    let file_type = detect_file_type(&file_path);
    assert_eq!(file_type, FileType::Hex);

    // Step 2: Get file size
    let file_size = fs::metadata(&file_path).expect("Failed to get metadata").len();
    let bytes = fs::read(&file_path).expect("Failed to read file");

    // Step 3: Create appropriate viewer
    let viewer = HexViewer::new_from_bytes(file_path.clone(), bytes, file_size)
        .expect("HexViewer creation should succeed");

    // Step 4: Get state and render
    let state = viewer.get_state();
    assert_eq!(state.file_type, "hex");
    let rendered = viewer.render().expect("Render should succeed");
    assert!(!rendered.is_empty());
}

#[test]
fn test_integration_complete_workflow_save_duplicate_create_note() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let original_file = vault.path().join("documents/notes.md");

    // Step 1: Read and detect file type
    let file_type = detect_file_type(&original_file);
    assert_eq!(file_type, FileType::Markdown);

    // Step 2: Read content and create viewer
    let content = fs::read_to_string(&original_file).expect("Failed to read file");
    let file_size = fs::metadata(&original_file).expect("Failed to get metadata").len();
    let viewer = MarkdownViewer::new(original_file.clone(), content, file_size);

    // Step 3: Verify we can get state
    let state = viewer.get_state();
    assert_eq!(state.file_type, "markdown");

    // Step 4: Duplicate the file
    let duplicated = file_ops::duplicate_file_impl(&original_file)
        .expect("Duplicate should succeed");
    assert!(duplicated.exists());

    // Step 5: Create a new note
    let documents_dir = vault.path().join("documents");
    let new_note = file_ops::create_markdown_note_impl(&documents_dir, "created_note.md")
        .expect("Note creation should succeed");
    assert!(new_note.exists());

    // Step 6: Verify all files are distinct
    assert_ne!(original_file, duplicated);
    assert_ne!(original_file, new_note);
    assert_ne!(duplicated, new_note);

    // Step 7: Read all files to verify they have expected content
    let original_content = fs::read_to_string(&original_file).expect("Read original");
    let dup_content = fs::read_to_string(&duplicated).expect("Read duplicate");
    let note_content = fs::read_to_string(&new_note).expect("Read new note");

    assert_eq!(dup_content, original_content);
    assert_eq!(note_content, "");
}

#[test]
fn test_integration_all_viewer_types_instantiate() {
    let vault = create_test_vault().expect("Failed to create test vault");

    // Markdown viewer
    let md_file = vault.path().join("README.md");
    let md_content = fs::read_to_string(&md_file).expect("Read markdown");
    let md_size = fs::metadata(&md_file).expect("Get markdown metadata").len();
    let _md_viewer = MarkdownViewer::new(md_file, md_content, md_size);

    // Code viewer
    let code_file = vault.path().join("code/hello.rs");
    let code_content = fs::read_to_string(&code_file).expect("Read code");
    let _code_viewer = CodeViewer::new(code_file, "rust".to_string(), code_content, false)
        .expect("CodeViewer creation should succeed");

    // Image viewer
    let img_file = vault.path().join("images/test.png");
    let _img_viewer = ImageViewer::new(img_file, "png".to_string(), 1, 1)
        .expect("ImageViewer creation should succeed");

    // Hex viewer
    let bin_file = vault.path().join("assets/data.bin");
    let bin_size = fs::metadata(&bin_file).expect("Get binary metadata").len();
    let bin_bytes = fs::read(&bin_file).expect("Read binary file");
    let _hex_viewer = HexViewer::new_from_bytes(bin_file, bin_bytes, bin_size)
        .expect("HexViewer creation should succeed");

    // If we reach here, all viewers instantiated successfully
}

#[test]
fn test_integration_all_file_operations() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let test_file = vault.path().join("documents/notes.md");

    // Test 1: Read and verify file exists
    assert!(test_file.exists());
    let content = fs::read_to_string(&test_file).expect("Read file");
    assert!(!content.is_empty());

    // Test 2: Duplicate file
    let dup1 = file_ops::duplicate_file_impl(&test_file).expect("First duplicate");
    assert!(dup1.exists());
    let dup1_content = fs::read_to_string(&dup1).expect("Read dup1");
    assert_eq!(dup1_content, content);

    // Test 3: Duplicate again
    let dup2 = file_ops::duplicate_file_impl(&test_file).expect("Second duplicate");
    assert!(dup2.exists());
    let dup2_content = fs::read_to_string(&dup2).expect("Read dup2");
    assert_eq!(dup2_content, content);

    // Test 4: Create new note
    let documents_dir = vault.path().join("documents");
    let note = file_ops::create_markdown_note_impl(&documents_dir, "workflow_test.md")
        .expect("Create note");
    assert!(note.exists());

    // Test 5: Verify all files are in vault
    let vault_files = fs::read_dir(&documents_dir).expect("Read directory");
    let file_count = vault_files.count();
    assert!(file_count >= 5); // original + 2 duplicates + 2 new notes at minimum
}

#[test]
fn test_integration_vault_with_sample_files() {
    let vault = create_test_vault().expect("Failed to create test vault");
    let vault_path = vault.path();

    // Verify main vault structure
    assert!(vault_path.exists());

    // Verify subdirectories
    let subdirs = ["documents", "code", "images", "assets"];
    for subdir in &subdirs {
        let dir_path = vault_path.join(subdir);
        assert!(dir_path.exists(), "Directory {} should exist", subdir);
        assert!(dir_path.is_dir(), "{} should be a directory", subdir);
    }

    // Verify sample files exist
    assert!(vault_path.join("README.md").exists());
    assert!(vault_path.join("documents/notes.md").exists());
    assert!(vault_path.join("code/hello.rs").exists());
    assert!(vault_path.join("code/script.py").exists());
    assert!(vault_path.join("images/test.png").exists());
    assert!(vault_path.join("assets/data.bin").exists());

    // Verify we can list all files
    let mut file_count = 0;
    for entry in walkdir::WalkDir::new(vault_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.path().is_file() {
            file_count += 1;
        }
    }
    assert!(file_count >= 12, "Should have at least 12 files in vault");
}
