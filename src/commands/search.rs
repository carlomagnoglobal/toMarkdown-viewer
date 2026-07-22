/// Full-text search functionality using SQLite FTS5
use std::path::Path;

/// Search files in the vault using FTS5.
///
/// Parameters:
/// - `vault_root`: Absolute path to the vault root directory
/// - `query`: Search query (FTS5 syntax supported)
/// - `search_content`: Whether to search file content; if false, search only path/name
///
/// Returns: Vec of matching file paths (absolute paths)
fn search_files_impl(vault_root: &Path, query: &str, search_content: bool) -> Result<Vec<String>, String> {
    use crate::vault::init_vault_db;

    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    // Initialize vault database
    let vault_db = init_vault_db(vault_root).map_err(|e| format!("Failed to init vault DB: {}", e))?;

    // Build FTS5 query: search across path, name, and optionally content
    let fts_query = if search_content {
        // Search all columns
        query.to_string()
    } else {
        // Search only path and name columns
        format!("path:{} OR name:{}", query, query)
    };

    // Prepare FTS5 search statement
    let mut stmt = vault_db
        .conn
        .prepare("SELECT path FROM files_fts WHERE files_fts MATCH ?1 LIMIT 100")
        .map_err(|e| format!("Failed to prepare search statement: {}", e))?;

    // Execute query and collect results
    let results = stmt
        .query_map([&fts_query], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Failed to execute search: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect results: {}", e))?;

    // Convert relative paths to absolute paths
    let absolute_results = results
        .into_iter()
        .map(|rel_path| vault_root.join(&rel_path).to_string_lossy().into_owned())
        .collect();

    Ok(absolute_results)
}

/// Tauri command to search files in a vault.
/// Accepts vault root path, search query, and search_content flag.
/// Returns list of absolute file paths matching the query.
#[tauri::command]
pub async fn search_files(
    vault_root: String,
    query: String,
    search_content: bool,
) -> Result<Vec<String>, String> {
    let root = Path::new(&vault_root);
    search_files_impl(root, &query, search_content)
}
