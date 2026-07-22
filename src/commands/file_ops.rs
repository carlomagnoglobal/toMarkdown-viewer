/// File operations: duplicate, move, create, etc.
use std::path::{Path, PathBuf};

/// Backend implementation: duplicate a file with smart naming.
///
/// Logic:
/// 1. Check file exists (error if not)
/// 2. Get parent directory
/// 3. Parse filename and extension
/// 4. Find available duplicate name:
///    - First try: `filename copy.ext`
///    - If exists: `filename copy (2).ext`, `filename copy (3).ext`, etc.
///    - Up to 100 attempts, then error
/// 5. Copy file via std::fs::copy
/// 6. Return PathBuf to new file
pub fn duplicate_file_impl(path: &Path) -> Result<PathBuf, String> {
    // Check file exists
    if !path.is_file() {
        return Err(format!("File not found: {}", path.display()));
    }

    // Get parent directory
    let parent = path.parent().unwrap_or(Path::new("."));

    // Parse filename and extension
    let file_name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| "Invalid filename".to_string())?;

    let (stem, ext) = match file_name.rsplit_once('.') {
        Some((s, e)) => (s.to_string(), format!(".{}", e)),
        None => (file_name.to_string(), String::new()),
    };

    // Find available duplicate name
    // First try: filename copy.ext
    let mut new_name = format!("{} copy{}", stem, ext);
    let mut new_path = parent.join(&new_name);

    // If it exists, try copy (2), copy (3), etc. up to 100 attempts
    let mut counter = 2;
    while new_path.exists() && counter <= 100 {
        new_name = format!("{} copy ({}){}", stem, counter, ext);
        new_path = parent.join(&new_name);
        counter += 1;
    }

    // If we exhausted all attempts, error
    if counter > 100 {
        return Err("Could not find available filename after 100 attempts".to_string());
    }

    // Copy file
    std::fs::copy(path, &new_path).map_err(|e| format!("Failed to copy file: {}", e))?;

    // Return PathBuf to new file
    Ok(new_path)
}

/// Tauri command to duplicate a file.
/// Accepts file path as String, returns path to duplicate as String.
#[tauri::command]
pub async fn duplicate_file(path: String) -> Result<String, String> {
    let p = PathBuf::from(&path);
    duplicate_file_impl(&p).map(|pb| pb.to_string_lossy().into_owned())
}

/// Backend implementation: create an empty markdown note in a folder.
///
/// Logic:
/// 1. Validate name is not empty (error if empty)
/// 2. Check folder exists as directory (error if not)
/// 3. Build note path: folder.join(name)
/// 4. Error if file already exists
/// 5. Create empty file at note_path via std::fs::write(..., "")
/// 6. Return PathBuf to new note
pub fn create_markdown_note_impl(folder: &Path, name: &str) -> Result<PathBuf, String> {
    // Validate name is not empty
    if name.trim().is_empty() {
        return Err("Note name cannot be empty".to_string());
    }

    // Check folder exists as directory
    if !folder.is_dir() {
        return Err(format!("Folder not found: {}", folder.display()));
    }

    // Build note path
    let note_path = folder.join(name);

    // Error if file already exists
    if note_path.exists() {
        return Err(format!("File already exists: {}", note_path.display()));
    }

    // Create empty file
    std::fs::write(&note_path, "").map_err(|e| format!("Failed to create file: {}", e))?;

    // Return PathBuf to new note
    Ok(note_path)
}

/// Tauri command to create a markdown note.
/// Accepts folder path and note name as Strings, returns path to new note as String.
#[tauri::command]
pub async fn create_markdown_note(folder: String, name: String) -> Result<String, String> {
    let f = PathBuf::from(&folder);
    create_markdown_note_impl(&f, &name).map(|pb| pb.to_string_lossy().into_owned())
}
