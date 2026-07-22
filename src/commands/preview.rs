/// File preview on hover: quick peeks of file content without opening.
use std::path::{Path, PathBuf};
use serde::Serialize;
use crate::file_types::{detect_file_type, FileType};

/// Preview data returned to the frontend.
#[derive(Serialize, Debug)]
pub struct FilePreview {
    /// File type: "markdown", "code", "image", or "other"
    pub kind: String,
    /// Preview content: truncated text or metadata
    pub content: String,
    /// Programming language (for code files)
    pub language: Option<String>,
    /// Whether content was truncated due to size limits
    pub truncated: bool,
}

/// Backend implementation: generate file preview.
///
/// Logic:
/// 1. Check file exists (error if not)
/// 2. Detect file type
/// 3. Generate preview based on type:
///    - Markdown: first 200 chars of content
///    - Code: first 10 lines of content
///    - Image: metadata (dimensions + format)
///    - Other: file info (size + extension)
/// 4. Return FilePreview struct
pub fn get_file_preview_impl(path: &Path) -> Result<FilePreview, String> {
    // Check file exists
    if !path.is_file() {
        return Err(format!("File not found: {}", path.display()));
    }

    let file_type = detect_file_type(path);

    match file_type {
        FileType::Markdown => {
            // Markdown: first 200 chars
            let content = std::fs::read_to_string(path)
                .map_err(|e| format!("Failed to read markdown file: {}", e))?;
            let truncated = content.len() > 200;
            let preview: String = content.chars().take(200).collect();
            Ok(FilePreview {
                kind: "markdown".to_string(),
                content: preview,
                language: None,
                truncated,
            })
        }

        FileType::Code { language } => {
            // Code: first 10 lines
            let content = std::fs::read_to_string(path)
                .map_err(|e| format!("Failed to read code file: {}", e))?;
            let lines: Vec<&str> = content.lines().collect();
            let truncated = lines.len() > 10;
            let preview = lines.iter().take(10).copied().collect::<Vec<_>>().join("\n");
            Ok(FilePreview {
                kind: "code".to_string(),
                content: preview,
                language: Some(language),
                truncated,
            })
        }

        FileType::Image { format } => {
            // Image: metadata (dimensions + format)
            let (width, height) = extract_image_dimensions(path).unwrap_or((0, 0));
            let size = std::fs::metadata(path)
                .map(|m| m.len())
                .unwrap_or(0);
            let content = format!(
                "{}x{} pixels, {} format, {:.1} KB",
                width, height, format, size as f64 / 1024.0
            );
            Ok(FilePreview {
                kind: "image".to_string(),
                content,
                language: None,
                truncated: false,
            })
        }

        FileType::Hex => {
            // Other/unknown: file info
            let size = std::fs::metadata(path)
                .map(|m| m.len())
                .unwrap_or(0);
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown");
            let content = format!("{} file, {:.1} KB", ext, size as f64 / 1024.0);
            Ok(FilePreview {
                kind: "other".to_string(),
                content,
                language: None,
                truncated: false,
            })
        }
    }
}

/// Extract image dimensions from a file using the image crate.
/// Returns (width, height) on success, None if dimensions cannot be determined.
fn extract_image_dimensions(path: &Path) -> Option<(u32, u32)> {
    use image::ImageReader;
    let reader = ImageReader::open(path).ok()?;
    let dimensions = reader.into_dimensions().ok()?;
    Some(dimensions)
}

/// Tauri command to get file preview on hover.
/// Accepts file path as String, returns FilePreview.
#[tauri::command]
pub async fn get_file_preview(path: String) -> Result<FilePreview, String> {
    let p = PathBuf::from(&path);
    get_file_preview_impl(&p)
}
