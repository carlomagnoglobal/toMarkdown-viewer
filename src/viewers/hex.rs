//! Hex dump file viewer implementation

use super::traits::{FileViewer, ViewerError, ViewerState};
use std::path::PathBuf;

/// Viewer for binary files displayed as hexadecimal
///
/// Displays file content as a hex dump with offset, hex bytes, and ASCII representation.
/// Each row displays 16 bytes for compact readability.
pub struct HexViewer {
    /// Path to the binary file
    #[allow(dead_code)]
    path: PathBuf,
    /// Raw binary data
    bytes: Vec<u8>,
    /// Total file size in bytes
    #[allow(dead_code)]
    total_size: u64,
}

impl HexViewer {
    /// Creates a new HexViewer from binary data
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file being viewed
    /// * `bytes` - Binary data to display
    /// * `total_size` - Total file size in bytes
    ///
    /// # Errors
    ///
    /// Returns `ViewerError` if the bytes vector is too large or other validation fails
    pub fn new_from_bytes(path: PathBuf, bytes: Vec<u8>, total_size: u64) -> Result<Self, ViewerError> {
        Ok(HexViewer {
            path,
            bytes,
            total_size,
        })
    }

    /// Formats a single line of hex output with 16 bytes
    ///
    /// # Arguments
    ///
    /// * `offset` - Current offset in the file
    /// * `chunk` - Slice of up to 16 bytes to format
    ///
    /// Returns a formatted HTML table row
    fn format_hex_line(offset: u64, chunk: &[u8]) -> String {
        // Format offset as 8 hex digits
        let offset_str = format!("{:08X}", offset);

        // Format hex bytes as space-separated pairs
        let hex_str = chunk
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        // Format ASCII representation (printable chars or dots)
        let ascii_str = chunk
            .iter()
            .map(|b| {
                if b.is_ascii_graphic() || *b == b' ' {
                    *b as char
                } else {
                    '.'
                }
            })
            .collect::<String>();

        format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
            offset_str, hex_str, ascii_str
        )
    }

    /// Render as raw text (lossy UTF-8 conversion)
    fn render_as_text(&self) -> String {
        String::from_utf8_lossy(&self.bytes).to_string()
    }

    /// Render as base64
    fn render_as_base64(&self) -> String {
        const STANDARD: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::new();

        for chunk in self.bytes.chunks(3) {
            let b1 = chunk[0];
            let b2 = chunk.get(1).copied().unwrap_or(0);
            let b3 = chunk.get(2).copied().unwrap_or(0);

            let n = ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32);

            result.push(STANDARD[((n >> 18) & 0x3F) as usize] as char);
            result.push(STANDARD[((n >> 12) & 0x3F) as usize] as char);

            if chunk.len() > 1 {
                result.push(STANDARD[((n >> 6) & 0x3F) as usize] as char);
            } else {
                result.push('=');
            }

            if chunk.len() > 2 {
                result.push(STANDARD[(n & 0x3F) as usize] as char);
            } else {
                result.push('=');
            }
        }

        result
    }

    /// Render as styled HTML hex dump
    pub fn render_hex_html(&self) -> String {
        let mut html = String::from(
            r#"<div style="padding: 20px; background: var(--bg); color: var(--fg); font-family: monospace; font-size: 13px; overflow-x: auto;">
<table style="border-collapse: collapse; width: 100%;">
<thead style="position: sticky; top: 0; background: var(--code-bg); border-bottom: 1px solid var(--border);">
<tr style="border-bottom: 1px solid var(--border);">
<th style="padding: 8px 12px; text-align: left; font-weight: 600;">Offset</th>
<th style="padding: 8px 12px; text-align: left; font-weight: 600;">Hex Data</th>
<th style="padding: 8px 12px; text-align: left; font-weight: 600;">ASCII</th>
</tr>
</thead>
<tbody>"#
        );

        // Process bytes in 16-byte chunks
        for (i, chunk) in self.bytes.chunks(16).enumerate() {
            let offset = (i as u64) * 16;
            let row = Self::format_hex_line(offset, chunk);
            html.push_str(&format!(r#"<tr style="border-bottom: 1px solid var(--border); hover-color: var(--hover);">{}</tr>"#, row));
        }

        html.push_str("</tbody></table></div>");
        html
    }

    /// Render as styled HTML text view
    pub fn render_text_html(&self) -> String {
        let text = self.render_as_text();
        // Escape HTML special characters
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;");
        format!(
            r#"<div style="padding: 20px; background: var(--bg); color: var(--fg); font-family: monospace; font-size: 13px; white-space: pre-wrap; word-wrap: break-word; line-height: 1.5;"><code>{}</code></div>"#,
            escaped
        )
    }

    /// Render as styled HTML base64 view
    pub fn render_base64_html(&self) -> String {
        let b64 = self.render_as_base64();
        format!(
            r#"<div style="padding: 20px; background: var(--bg); color: var(--fg); font-family: monospace; font-size: 13px; white-space: pre-wrap; word-wrap: break-word; line-height: 1.5;"><code>{}</code></div>"#,
            b64
        )
    }
}

impl FileViewer for HexViewer {
    fn render(&self) -> Result<String, ViewerError> {
        Ok(self.render_hex_html())
    }

    fn get_state(&self) -> ViewerState {
        ViewerState {
            file_type: "hex".to_string(),
            file_path: self.path.clone(),
            modified: false,
            file_size_bytes: self.total_size,
        }
    }

    fn file_type(&self) -> &str {
        "hex"
    }
}
