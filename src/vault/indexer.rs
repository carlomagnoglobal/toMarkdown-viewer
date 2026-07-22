//! Vault indexing module for efficient file tracking and search
//!
//! Provides lazy indexing of vault files with metadata storage:
//! - File type detection
//! - Metadata extraction (size, modification time, language)
//! - Database insertion for search and relationship tracking

use super::{VaultDb, schema};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir;
use crate::file_types::{detect_file_type, FileType};

/// Index a single file into the vault database.
///
/// Reads file metadata and inserts/updates the file entry in the database
/// with: path, name, extension, file_type, language, size, modified time.
///
/// # Arguments
/// * `vault_db` - Vault database connection
/// * `file_path` - Absolute path to the file to index
/// * `vault_root` - Root directory of the vault
///
/// # Returns
/// `Ok(())` on success, error if file metadata cannot be read or DB insert fails
pub fn index_file(vault_db: &VaultDb, file_path: &Path, vault_root: &Path) -> Result<(), String> {
    // Verify file exists
    if !file_path.is_file() {
        return Err(format!("File not found: {}", file_path.display()));
    }

    // Compute relative path from vault root
    let rel_path = file_path
        .strip_prefix(vault_root)
        .map_err(|_| "File is outside vault root".to_string())?
        .to_string_lossy()
        .into_owned();

    // Get file metadata
    let metadata = std::fs::metadata(file_path)
        .map_err(|e| format!("Failed to read metadata: {}", e))?;

    let file_size = metadata.len();
    let modified_time = metadata
        .modified()
        .unwrap_or(SystemTime::now())
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // Detect file type
    let (file_type_str, language) = match detect_file_type(file_path) {
        FileType::Markdown => ("markdown".to_string(), None),
        FileType::Code { language } => ("code".to_string(), Some(language)),
        FileType::Image { format } => (format!("image/{}", format), None),
        FileType::Hex => ("binary".to_string(), None),
    };

    // Get file name and extension
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let extension = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();

    // Insert into database
    vault_db
        .conn
        .execute(
            "INSERT OR REPLACE INTO files (path, name, extension, file_type, language, size, modified_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &rel_path,
                &file_name,
                &extension,
                &file_type_str,
                &language,
                file_size as i64,
                modified_time,
                modified_time,
            ],
        )
        .map_err(|e| format!("Failed to insert file into database: {}", e))?;

    Ok(())
}

/// Index the entire vault, walking all files recursively.
///
/// Walks the vault directory tree and indexes all files, logging progress
/// and final count. Skips hidden directories and common build artifacts.
///
/// # Arguments
/// * `vault_db` - Vault database connection
///
/// # Returns
/// Number of files indexed, or error if walk fails
pub fn index_vault(vault_db: &VaultDb) -> Result<usize, String> {
    let vault_root = vault_db.vault_root.clone();
    let start_time = std::time::Instant::now();

    tracing::info!("Starting vault indexing for: {}", vault_root.display());

    // Common directories to skip
    const SKIP_DIRS: &[&str] = &[".git", ".obsidian", ".tomarkdown", "node_modules", "target"];

    let mut file_count = 0;

    // Walk the vault directory
    let walker = walkdir::WalkDir::new(&vault_root)
        .into_iter()
        .filter_entry(|entry| {
            // Skip excluded directories
            if entry.file_type().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    return !SKIP_DIRS.contains(&name);
                }
            }
            true
        })
        .filter_map(|e| e.ok());

    for entry in walker {
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Index file
        match index_file(vault_db, path, &vault_root) {
            Ok(_) => {
                file_count += 1;
                tracing::debug!("Indexed file: {}", path.display());
            }
            Err(e) => {
                tracing::warn!("Failed to index {}: {}", path.display(), e);
            }
        }
    }

    let elapsed = start_time.elapsed();
    tracing::info!(
        "Vault indexing complete: {} files indexed in {:.2}s",
        file_count,
        elapsed.as_secs_f64()
    );

    Ok(file_count)
}
