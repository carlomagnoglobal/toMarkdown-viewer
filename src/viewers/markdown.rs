//! Markdown file viewer implementation

use super::traits::{FileViewer, ViewerError, ViewerState};
use std::path::PathBuf;

/// Viewer for Markdown files
///
/// Displays pre-rendered HTML from markdown content.
/// Read-only viewer that wraps existing markdown rendering logic.
#[allow(dead_code)]
pub struct MarkdownViewer {
    /// Path to the markdown file
    path: PathBuf,
    /// Pre-rendered HTML content
    html: String,
    /// File size in bytes
    file_size: u64,
}

impl MarkdownViewer {
    /// Creates a new MarkdownViewer
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the markdown file
    /// * `html` - Pre-rendered HTML content
    /// * `file_size` - File size in bytes
    #[allow(dead_code)]
    pub fn new(path: PathBuf, html: String, file_size: u64) -> Self {
        MarkdownViewer {
            path,
            html,
            file_size,
        }
    }
}

impl FileViewer for MarkdownViewer {
    fn render(&self) -> Result<String, ViewerError> {
        Ok(self.html.clone())
    }

    fn get_state(&self) -> ViewerState {
        ViewerState {
            file_type: "markdown".to_string(),
            file_path: self.path.clone(),
            modified: false,
            file_size_bytes: self.file_size,
        }
    }

    fn file_type(&self) -> &str {
        "markdown"
    }
}
