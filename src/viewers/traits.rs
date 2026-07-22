//! File viewer trait and supporting types for polymorphic viewer system

use serde::Serialize;
use std::error::Error;
use std::fmt;
use std::path::PathBuf;

/// Error type for viewer operations
#[derive(Clone, Debug)]
pub enum ViewerError {
    /// File I/O errors
    IoError(String),
    /// Content parsing errors
    ParseError(String),
    /// Rendering errors
    #[allow(dead_code)]
    RenderError(String),
}

impl fmt::Display for ViewerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViewerError::IoError(msg) => write!(f, "I/O error: {}", msg),
            ViewerError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ViewerError::RenderError(msg) => write!(f, "Render error: {}", msg),
        }
    }
}

impl Error for ViewerError {}

/// State information for a file viewer
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize)]
pub struct ViewerState {
    /// Type identifier (markdown, code, image, hex)
    pub file_type: String,
    /// Full path to file
    pub file_path: PathBuf,
    /// Whether content is unsaved
    pub modified: bool,
    /// File size in bytes
    pub file_size_bytes: u64,
}

/// Trait for polymorphic file viewer implementations
#[allow(dead_code)]
pub trait FileViewer: Send + Sync {
    /// Returns HTML or formatted content for display
    fn render(&self) -> Result<String, ViewerError>;

    /// Returns current viewer state
    fn get_state(&self) -> ViewerState;

    /// Returns viewer type identifier
    fn file_type(&self) -> &str;
}
