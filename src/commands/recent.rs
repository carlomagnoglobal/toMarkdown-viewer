/// Recent files tracking and retrieval.
use std::path::{Path, PathBuf};
use serde::Serialize;

/// Metadata for a recently opened file.
#[derive(Serialize, Debug, Clone)]
pub struct RecentFile {
    /// Absolute file path
    pub path: String,
    /// File name (basename)
    pub name: String,
    /// Unix timestamp when file was last opened
    pub opened_at: i64,
}

/// Backend implementation: fetch recently opened files.
///
/// Logic:
/// 1. Initialize vault database at vault_root
/// 2. Query recent_files table ordered by opened_at DESC
/// 3. Join with files table to get full path and name
/// 4. Return last 10 records
/// 5. Return empty list if no recent files exist
pub fn get_recent_files_impl(vault_root: &Path) -> Result<Vec<RecentFile>, String> {
    use crate::vault::init_vault_db;
    let db = init_vault_db(vault_root).map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn
        .prepare(
            r#"
        SELECT
            f.path,
            f.name,
            rf.opened_at
        FROM recent_files rf
        INNER JOIN files f ON rf.file_id = f.id
        ORDER BY rf.opened_at DESC
        LIMIT 10
        "#,
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let recent = stmt
        .query_map([], |row| {
            Ok(RecentFile {
                path: row.get(0)?,
                name: row.get(1)?,
                opened_at: row.get(2)?,
            })
        })
        .map_err(|e| format!("Failed to query recent files: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to read recent files: {}", e))?;

    Ok(recent)
}

/// Backend implementation: update recent file tracking.
///
/// Logic:
/// 1. Initialize vault database at vault_root
/// 2. Add file to files table if not exists (or update if exists)
/// 3. Insert/update recent_files entry with current timestamp
/// 4. Error if file path is outside vault or doesn't exist
pub fn update_recent_file_impl(vault_root: &Path, file_path: &Path) -> Result<(), String> {
    use crate::vault::init_vault_db;

    // Verify file exists
    if !file_path.is_file() {
        return Err(format!("File not found: {}", file_path.display()));
    }

    // Verify file is within vault
    let abs_vault = vault_root
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize vault root: {}", e))?;
    let abs_file = file_path
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize file path: {}", e))?;

    if !abs_file.starts_with(&abs_vault) {
        return Err("File is outside the vault".to_string());
    }

    let db = init_vault_db(vault_root).map_err(|e| e.to_string())?;

    // Compute relative path from vault root
    let rel_path = abs_file
        .strip_prefix(&abs_vault)
        .map_err(|_| "Failed to compute relative path")?
        .to_string_lossy()
        .into_owned();

    let file_name = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // Insert or ignore file record (if already exists, just ignore)
    db.conn
        .execute(
            r#"
        INSERT OR IGNORE INTO files (path, name, extension, modified_at, is_indexed)
        VALUES (?, ?, ?, ?, 0)
        "#,
            [
                &rel_path,
                &file_name,
                &file_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_string(),
                &current_time.to_string(),
            ],
        )
        .map_err(|e| format!("Failed to insert file record: {}", e))?;

    // Get the file_id
    let file_id: i64 = db
        .conn
        .query_row(
            "SELECT id FROM files WHERE path = ?",
            [&rel_path],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to get file id: {}", e))?;

    // Insert or replace recent_files entry
    db.conn
        .execute(
            "INSERT OR REPLACE INTO recent_files (file_id, opened_at) VALUES (?, ?)",
            [file_id.to_string(), current_time.to_string()],
        )
        .map_err(|e| format!("Failed to update recent files: {}", e))?;

    Ok(())
}

/// Tauri command to get recent files.
/// Accepts vault root path as String, returns Vec of RecentFile.
#[tauri::command]
pub async fn get_recent_files(vault_root: String) -> Result<Vec<RecentFile>, String> {
    let vr = PathBuf::from(&vault_root);
    get_recent_files_impl(&vr)
}

/// Tauri command to update recent file tracking.
/// Accepts vault root and file path as Strings.
#[tauri::command]
pub async fn update_recent_file(vault_root: String, file_path: String) -> Result<(), String> {
    let vr = PathBuf::from(&vault_root);
    let fp = PathBuf::from(&file_path);
    update_recent_file_impl(&vr, &fp)
}
