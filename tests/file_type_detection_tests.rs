use std::path::Path;
use to_markdown_gui::file_types::{detect_file_type, FileType};

#[test]
fn test_detect_markdown() {
    let path = Path::new("example.md");
    match detect_file_type(path) {
        FileType::Markdown => (),
        _ => panic!("Expected Markdown variant for .md file"),
    }
}

#[test]
fn test_detect_python_code() {
    let path = Path::new("script.py");
    match detect_file_type(path) {
        FileType::Code { language } => {
            assert_eq!(language, "python", "Expected language to be 'python' for .py file");
        }
        _ => panic!("Expected Code variant for .py file"),
    }
}

#[test]
fn test_detect_rust_code() {
    let path = Path::new("main.rs");
    match detect_file_type(path) {
        FileType::Code { language } => {
            assert_eq!(language, "rust", "Expected language to be 'rust' for .rs file");
        }
        _ => panic!("Expected Code variant for .rs file"),
    }
}

#[test]
fn test_detect_png_image() {
    let path = Path::new("image.png");
    match detect_file_type(path) {
        FileType::Image { format } => {
            assert_eq!(format, "png", "Expected format to be 'png' for .png file");
        }
        _ => panic!("Expected Image variant for .png file"),
    }
}

#[test]
fn test_detect_unknown_as_hex() {
    let path = Path::new("binary.bin");
    match detect_file_type(path) {
        FileType::Hex => (),
        _ => panic!("Expected Hex variant for .bin file"),
    }
}
