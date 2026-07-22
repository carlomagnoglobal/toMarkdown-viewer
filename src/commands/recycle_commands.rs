/// Recycle bin operations: delete, restore, and manage deleted files.
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

use crate::state::{VaultViewerState, DeletedFile};

/// Tauri command to delete a file by moving it to the recycle bin.
///
/// Arguments:
/// - path: Absolute path to the file to delete
/// - vault_root: Root path of the vault
/// - state: Application state containing recycle bin management
///
/// Returns: Ok(()) on success, Err on failure
#[tauri::command]
pub async fn delete_file(
    path: String,
    vault_root: String,
    state: State<'_, Mutex<VaultViewerState>>,
) -> Result<(), String> {
    let vault_root_path = PathBuf::from(&vault_root);
    let mut viewer_state = state.lock().map_err(|e| format!("Failed to lock state: {}", e))?;
    viewer_state.delete_file(path, &vault_root_path)
}

/// Tauri command to restore a file from the recycle bin.
///
/// Arguments:
/// - file_id: Unique identifier of the deleted file
/// - vault_root: Root path of the vault
/// - state: Application state containing recycle bin management
///
/// Returns: Ok(()) on success, Err on failure (e.g., file not found)
#[tauri::command]
pub async fn restore_file(
    file_id: String,
    vault_root: String,
    state: State<'_, Mutex<VaultViewerState>>,
) -> Result<(), String> {
    let vault_root_path = PathBuf::from(&vault_root);
    let mut viewer_state = state.lock().map_err(|e| format!("Failed to lock state: {}", e))?;
    viewer_state.restore_file(&file_id, &vault_root_path)
}

/// Tauri command to permanently delete a file from the recycle bin.
///
/// Arguments:
/// - file_id: Unique identifier of the deleted file
/// - vault_root: Root path of the vault
/// - state: Application state containing recycle bin management
///
/// Returns: Ok(()) on success, Err on failure (e.g., file not found)
#[tauri::command]
pub async fn permanently_delete(
    file_id: String,
    vault_root: String,
    state: State<'_, Mutex<VaultViewerState>>,
) -> Result<(), String> {
    let vault_root_path = PathBuf::from(&vault_root);
    let mut viewer_state = state.lock().map_err(|e| format!("Failed to lock state: {}", e))?;
    viewer_state.permanently_delete(&file_id, &vault_root_path)
}

/// Tauri command to retrieve all deleted files from the recycle bin.
///
/// Returns: Vec<DeletedFile> containing metadata for all deleted files
#[tauri::command]
pub async fn get_deleted_files(
    state: State<'_, Mutex<VaultViewerState>>,
) -> Result<Vec<DeletedFile>, String> {
    let viewer_state = state.lock().map_err(|e| format!("Failed to lock state: {}", e))?;
    Ok(viewer_state.deleted_files.clone())
}

/// Tauri command to empty the recycle bin by permanently deleting all files.
///
/// Arguments:
/// - vault_root: Root path of the vault
/// - state: Application state containing recycle bin management
///
/// Returns: Ok(()) on success, Err on failure
#[tauri::command]
pub async fn empty_recycle_bin(
    vault_root: String,
    state: State<'_, Mutex<VaultViewerState>>,
) -> Result<(), String> {
    let vault_root_path = PathBuf::from(&vault_root);
    let mut viewer_state = state.lock().map_err(|e| format!("Failed to lock state: {}", e))?;

    // Clone the list of file IDs to avoid borrow conflicts
    let file_ids: Vec<String> = viewer_state.deleted_files.iter().map(|f| f.id.clone()).collect();

    // Delete each file
    for file_id in file_ids {
        viewer_state.permanently_delete(&file_id, &vault_root_path)?;
    }

    Ok(())
}
