/// Tests for file operations (duplicate_file, etc.)
use std::path::Path;
use tempfile::TempDir;

// Access to the library functions
use to_markdown_gui::commands::file_ops;

#[test]
fn test_duplicate_file() {
    // Create a temporary directory for testing
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Create a source file with specific content
    let src_path = temp_path.join("test_file.txt");
    let original_content = b"Hello, World!";
    std::fs::write(&src_path, original_content).expect("Failed to write source file");

    // Call duplicate_file_impl
    let dup_path = file_ops::duplicate_file_impl(&src_path).expect("duplicate_file_impl should succeed");

    // Verify the duplicate file was created
    assert!(dup_path.exists(), "Duplicate file should exist");

    // Verify the name pattern is correct
    assert_eq!(dup_path.file_name().unwrap(), "test_file copy.txt");

    // Verify the content matches the original
    let dup_content = std::fs::read(&dup_path).expect("Failed to read duplicate file");
    assert_eq!(dup_content, original_content, "Content should match");

    // Verify both files exist
    assert!(src_path.exists(), "Original file should still exist");
    assert!(dup_path.exists(), "Duplicate file should exist");
}

#[test]
fn test_duplicate_file_numbering() {
    // Create a temporary directory for testing
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Create the original file
    let src_path = temp_path.join("document.md");
    std::fs::write(&src_path, b"# Original Document").expect("Failed to write original");

    // First duplicate: should be "document copy.md"
    let dup1 = file_ops::duplicate_file_impl(&src_path).expect("First duplicate should succeed");
    assert_eq!(dup1.file_name().unwrap(), "document copy.md");
    assert!(dup1.exists());

    // Second duplicate: should be "document copy (2).md"
    let dup2 = file_ops::duplicate_file_impl(&src_path).expect("Second duplicate should succeed");
    assert_eq!(dup2.file_name().unwrap(), "document copy (2).md");
    assert!(dup2.exists());

    // Third duplicate: should be "document copy (3).md"
    let dup3 = file_ops::duplicate_file_impl(&src_path).expect("Third duplicate should succeed");
    assert_eq!(dup3.file_name().unwrap(), "document copy (3).md");
    assert!(dup3.exists());

    // Verify all files have the same content
    let original_content = std::fs::read(&src_path).expect("Failed to read original");
    let dup1_content = std::fs::read(&dup1).expect("Failed to read dup1");
    let dup2_content = std::fs::read(&dup2).expect("Failed to read dup2");
    let dup3_content = std::fs::read(&dup3).expect("Failed to read dup3");

    assert_eq!(dup1_content, original_content);
    assert_eq!(dup2_content, original_content);
    assert_eq!(dup3_content, original_content);
}

#[test]
fn test_duplicate_file_missing_file() {
    // Try to duplicate a file that doesn't exist
    let nonexistent = Path::new("/tmp/surely_does_not_exist_12345.txt");
    let result = file_ops::duplicate_file_impl(nonexistent);
    assert!(result.is_err(), "Should error when file doesn't exist");
}

#[test]
fn test_duplicate_file_no_extension() {
    // Create a temporary directory for testing
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Create a file with no extension
    let src_path = temp_path.join("README");
    std::fs::write(&src_path, b"Read me!").expect("Failed to write file");

    // Duplicate it
    let dup_path = file_ops::duplicate_file_impl(&src_path).expect("Duplicate should succeed");

    // Verify the name pattern is correct (no extension case)
    assert_eq!(dup_path.file_name().unwrap(), "README copy");
    assert!(dup_path.exists());
}

#[test]
fn test_create_markdown_note() {
    // Create a temporary directory for testing
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Create a markdown note
    let note_name = "test_note.md";
    let note_path = file_ops::create_markdown_note_impl(temp_path, note_name)
        .expect("create_markdown_note_impl should succeed");

    // Verify the file was created
    assert!(note_path.exists(), "Note file should exist");

    // Verify the name is correct
    assert_eq!(note_path.file_name().unwrap(), note_name);

    // Verify the content is empty
    let content = std::fs::read_to_string(&note_path).expect("Failed to read note file");
    assert_eq!(content, "", "Note should be empty");

    // Verify the file is in the correct folder
    assert_eq!(note_path.parent().unwrap(), temp_path);
}

#[test]
fn test_create_markdown_note_invalid_name() {
    // Create a temporary directory for testing
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Try to create a note with an empty name
    let result = file_ops::create_markdown_note_impl(temp_path, "");
    assert!(result.is_err(), "Should error when name is empty");

    // Try with whitespace-only name
    let result = file_ops::create_markdown_note_impl(temp_path, "   ");
    assert!(result.is_err(), "Should error when name is only whitespace");
}
