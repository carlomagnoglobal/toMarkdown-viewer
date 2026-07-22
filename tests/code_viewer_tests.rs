use std::path::PathBuf;
use to_markdown_gui::viewers::{CodeViewer, FileViewer};

#[test]
fn test_code_viewer_creates() {
    let path = PathBuf::from("test.rs");
    let language = "rust".to_string();
    let content = "fn main() {\n    println!(\"Hello, world!\");\n}".to_string();
    let dirty = false;

    let viewer = CodeViewer::new(path.clone(), language, content.clone(), dirty)
        .expect("Failed to create CodeViewer");

    // Verify the viewer was created successfully
    assert_eq!(viewer.get_content(), content, "Content should match");
    assert!(!viewer.is_dirty(), "Viewer should not be dirty initially");
    assert_eq!(viewer.file_type(), "code", "File type should be 'code'");
}

#[test]
fn test_code_viewer_render() {
    let path = PathBuf::from("test.rs");
    let language = "rust".to_string();
    let content = "fn main() {\n    println!(\"Hello\");\n}".to_string();
    let dirty = false;

    let viewer = CodeViewer::new(path, language, content, dirty)
        .expect("Failed to create CodeViewer");

    let html = viewer.render().expect("Failed to render");

    // Verify HTML structure
    assert!(html.contains("<pre"), "HTML should contain <pre> tag");
    assert!(html.contains("</pre>"), "HTML should contain closing </pre> tag");

    // Verify line numbers are present
    assert!(html.contains("class=\"line"), "HTML should contain line number classes");

    // Verify content is present
    assert!(html.contains("main"), "HTML should contain code content");
}

#[test]
fn test_code_viewer_dirty_flag() {
    let path = PathBuf::from("test.rs");
    let language = "rust".to_string();
    let content = "fn main() {}".to_string();
    let dirty = false;

    let mut viewer = CodeViewer::new(path, language, content, dirty)
        .expect("Failed to create CodeViewer");

    // Initial state should not be dirty
    assert!(!viewer.is_dirty(), "Viewer should not be dirty initially");

    let state = viewer.get_state();
    assert!(!state.modified, "State should show not modified");

    // Set dirty flag
    viewer.set_dirty(true);
    assert!(viewer.is_dirty(), "Viewer should be dirty after set_dirty(true)");

    let state = viewer.get_state();
    assert!(state.modified, "State should show modified");
    assert_eq!(state.file_type, "code", "State file_type should be 'code'");
}

#[test]
fn test_code_viewer_update_content() {
    let path = PathBuf::from("test.rs");
    let language = "rust".to_string();
    let original_content = "fn main() {}".to_string();
    let dirty = false;

    let mut viewer = CodeViewer::new(path, language, original_content.clone(), dirty)
        .expect("Failed to create CodeViewer");

    // Initial state should not be dirty
    assert!(!viewer.is_dirty(), "Viewer should not be dirty initially");
    assert_eq!(viewer.get_content(), original_content, "Content should match original");

    // Update content
    let new_content = "fn main() {\n    println!(\"updated\");\n}".to_string();
    viewer.update_content(new_content.clone());

    // Verify content was updated and dirty flag is set
    assert_eq!(viewer.get_content(), new_content, "Content should match updated value");
    assert!(viewer.is_dirty(), "Viewer should be dirty after update_content");
}

#[test]
fn test_code_viewer_save_clears_dirty() {
    let path = PathBuf::from("test.rs");
    let language = "rust".to_string();
    let content = "fn main() {}".to_string();
    let dirty = false;

    let mut viewer = CodeViewer::new(path, language, content.clone(), dirty)
        .expect("Failed to create CodeViewer");

    // Update content to make it dirty
    let new_content = "fn main() {\n    println!(\"hello\");\n}".to_string();
    viewer.update_content(new_content.clone());
    assert!(viewer.is_dirty(), "Viewer should be dirty after update");

    // Save content
    let result = viewer.save_content();
    assert!(result.is_ok(), "save_content should succeed");
    assert!(!viewer.is_dirty(), "Viewer should not be dirty after save_content");
    assert_eq!(viewer.get_content(), new_content, "Content should remain unchanged after save");
}
