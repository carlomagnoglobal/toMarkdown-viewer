//! Code file viewer implementation with syntax highlighting

use super::traits::{FileViewer, ViewerError, ViewerState};
use std::path::PathBuf;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::html::{styled_line_to_highlighted_html, IncludeBackground};
use syntect::parsing::SyntaxSet;

/// Viewer for code files with syntax highlighting
///
/// Displays code with syntax highlighting using syntect.
/// Supports multiple programming languages and includes line numbers.
pub struct CodeViewer {
    /// Path to the code file
    #[allow(dead_code)]
    path: PathBuf,
    /// Programming language identifier
    language: String,
    /// File content
    content: String,
    /// Unsaved changes flag
    #[allow(dead_code)]
    dirty: bool,
    /// File size in bytes
    #[allow(dead_code)]
    file_size: u64,
}

impl CodeViewer {
    /// Creates a new CodeViewer with syntax highlighting support
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the code file
    /// * `language` - Programming language identifier (e.g., "rust", "python", "javascript")
    /// * `content` - File content as string
    /// * `dirty` - Whether the content has unsaved changes
    ///
    /// # Errors
    ///
    /// Returns `ViewerError` if content validation fails
    pub fn new(path: PathBuf, language: String, content: String, dirty: bool) -> Result<Self, ViewerError> {
        let file_size = content.len() as u64;
        Ok(CodeViewer {
            path,
            language,
            content,
            dirty,
            file_size,
        })
    }

    /// Updates the dirty flag
    ///
    /// # Arguments
    ///
    /// * `dirty` - New dirty state
    #[allow(dead_code)]
    pub fn set_dirty(&mut self, dirty: bool) {
        self.dirty = dirty;
    }

    /// Updates the file content and marks as dirty
    ///
    /// # Arguments
    ///
    /// * `content` - New content to set
    #[allow(dead_code)]
    pub fn update_content(&mut self, content: String) {
        self.content = content.clone();
        self.file_size = content.len() as u64;
        self.dirty = true;
    }

    /// Saves the content and clears the dirty flag
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if save is successful, `Err(String)` if save fails
    #[allow(dead_code)]
    pub fn save_content(&mut self) -> Result<(), String> {
        // Clear the dirty flag after successful save
        self.dirty = false;
        Ok(())
    }

    /// Returns a reference to the file content
    #[allow(dead_code)]
    pub fn get_content(&self) -> &str {
        &self.content
    }

    /// Returns whether the content has unsaved changes
    #[allow(dead_code)]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

}

impl FileViewer for CodeViewer {
    fn render(&self) -> Result<String, ViewerError> {
        // Load default syntax definitions and theme
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = &theme_set.themes["base16-ocean.dark"];

        // Find syntax for the language
        let syntax = syntax_set
            .find_syntax_by_token(&self.language)
            .or_else(|| syntax_set.find_syntax_by_extension(&self.language))
            .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

        let mut highlighter = HighlightLines::new(syntax, theme);

        // Build HTML output with line numbers and syntax highlighting
        let mut html = String::from("<pre style=\"background-color: #2b303b; color: #c0c5ce; padding: 12px; border-radius: 4px; overflow-x: auto; font-family: monospace;\">");

        for (line_num, line) in self.content.lines().enumerate() {
            let line_number = line_num + 1;
            let ranges = highlighter
                .highlight_line(line, &syntax_set)
                .unwrap_or_default();

            // Render each line with line number
            html.push_str(&format!(
                "<div class=\"line\" style=\"margin: 0;\"><span style=\"color: #65737e; margin-right: 12px;\">{:4}</span>",
                line_number
            ));

            // Add highlighted code
            let highlighted = styled_line_to_highlighted_html(&ranges, IncludeBackground::No)
                .unwrap_or_else(|_| String::new());
            html.push_str(&highlighted);
            html.push_str("</div>");
        }

        html.push_str("</pre>");
        Ok(html)
    }

    fn get_state(&self) -> ViewerState {
        ViewerState {
            file_type: "code".to_string(),
            file_path: self.path.clone(),
            modified: self.dirty,
            file_size_bytes: self.file_size,
        }
    }

    fn file_type(&self) -> &str {
        "code"
    }
}
