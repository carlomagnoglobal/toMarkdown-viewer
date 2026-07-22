/// Tab management commands: add, close, navigate, and retrieve tab state.
use std::path::Path;
use std::sync::Mutex;
use tauri::State;
use serde::Serialize;

use crate::state::{VaultViewerState, Tab};
use crate::config::save_tabs;

/// Serializable Tab struct for frontend communication.
/// Mirrors crate::state::Tab but explicitly defines serialization.
#[derive(Debug, Clone, Serialize)]
pub struct TabInfo {
    pub id: String,
    pub path: String,
    pub title: String,
    pub is_dirty: bool,
    pub tab_type: String,
}

/// Convert from internal Tab to serializable TabInfo.
impl From<crate::state::Tab> for TabInfo {
    fn from(tab: Tab) -> Self {
        TabInfo {
            id: tab.id,
            path: tab.path,
            title: tab.title,
            is_dirty: tab.is_dirty,
            tab_type: tab.tab_type,
        }
    }
}

/// Tauri command to add a new tab.
///
/// Arguments:
/// - path: Absolute file path
/// - title: Display title for the tab
/// - tab_type: File type ("markdown", "code", "image", "hex")
///
/// Returns: ID of the newly created tab
/// After adding, saves the tabs to disk if vault_root is available.
#[tauri::command]
pub async fn add_tab(
    path: String,
    title: String,
    tab_type: String,
    state: State<'_, Mutex<VaultViewerState>>,
) -> Result<String, String> {
    let mut viewer_state = state.lock().map_err(|e| format!("Failed to lock state: {}", e))?;
    let tab_id = viewer_state.add_tab(path, title, tab_type);

    // Persist tabs to disk if vault_root is available
    if let Some(vault_root) = &viewer_state.vault_root {
        let _ = save_tabs(
            Path::new(vault_root),
            &viewer_state.open_tabs,
            &viewer_state.active_tab,
        );
    }

    Ok(tab_id)
}

/// Tauri command to close a tab.
///
/// Arguments:
/// - tab_id: ID of the tab to close
///
/// If the closed tab was active, switches to the first remaining tab.
/// After closing, saves the tabs to disk if vault_root is available.
/// Returns an error if no tab with the given ID exists.
#[tauri::command]
pub async fn close_tab(
    tab_id: String,
    state: State<'_, Mutex<VaultViewerState>>,
) -> Result<(), String> {
    let mut viewer_state = state.lock().map_err(|e| format!("Failed to lock state: {}", e))?;
    viewer_state.close_tab(&tab_id);

    // Persist tabs to disk if vault_root is available
    if let Some(vault_root) = &viewer_state.vault_root {
        let _ = save_tabs(
            Path::new(vault_root),
            &viewer_state.open_tabs,
            &viewer_state.active_tab,
        );
    }

    Ok(())
}

/// Tauri command to set the active tab.
///
/// Arguments:
/// - tab_id: ID of the tab to activate
///
/// Pushes the previous active tab to the back/history stack.
/// After changing, saves the tabs to disk if vault_root is available.
/// Returns an error if the tab ID doesn't exist.
#[tauri::command]
pub async fn set_active_tab(
    tab_id: String,
    state: State<'_, Mutex<VaultViewerState>>,
) -> Result<(), String> {
    let mut viewer_state = state.lock().map_err(|e| format!("Failed to lock state: {}", e))?;
    viewer_state.set_active_tab(tab_id);

    // Persist tabs to disk if vault_root is available
    if let Some(vault_root) = &viewer_state.vault_root {
        let _ = save_tabs(
            Path::new(vault_root),
            &viewer_state.open_tabs,
            &viewer_state.active_tab,
        );
    }

    Ok(())
}

/// Tauri command to navigate back to the previous tab.
///
/// Pops the most recent tab from the back/history stack and activates it.
/// Does nothing if the history is empty.
#[tauri::command]
pub async fn back_button(
    state: State<'_, Mutex<VaultViewerState>>,
) -> Result<(), String> {
    let mut viewer_state = state.lock().map_err(|e| format!("Failed to lock state: {}", e))?;
    viewer_state.back();
    Ok(())
}

/// Tauri command to retrieve all open tabs and active tab info.
///
/// Returns:
/// - open_tabs: Vec of TabInfo for all open tabs
/// - active_tab: ID of the currently active tab (or None if no tabs)
#[tauri::command]
pub async fn get_tabs(
    state: State<'_, Mutex<VaultViewerState>>,
) -> Result<GetTabsResponse, String> {
    let viewer_state = state.lock().map_err(|e| format!("Failed to lock state: {}", e))?;

    let open_tabs = viewer_state
        .open_tabs
        .iter()
        .map(|tab| TabInfo::from(tab.clone()))
        .collect();

    let active_tab = viewer_state.active_tab.clone();

    Ok(GetTabsResponse {
        open_tabs,
        active_tab,
    })
}

/// Response struct for get_tabs command.
#[derive(Debug, Serialize)]
pub struct GetTabsResponse {
    pub open_tabs: Vec<TabInfo>,
    pub active_tab: Option<String>,
}

/// Tauri command to set the vault root path.
///
/// This is called when the user opens a vault/folder.
/// The vault root is used for loading and saving tab state.
///
/// Arguments:
/// - vault_root: Path to the vault root directory
#[tauri::command]
pub async fn set_vault_root(
    vault_root: String,
    state: State<'_, Mutex<VaultViewerState>>,
) -> Result<(), String> {
    let mut viewer_state = state.lock().map_err(|e| format!("Failed to lock state: {}", e))?;
    viewer_state.vault_root = Some(vault_root);
    Ok(())
}

/// Tauri command to load saved tabs for the current vault.
///
/// Loads tabs from `.tomarkdown/tabs.json` if the file exists and
/// the `auto_restore_tabs` preference is enabled.
///
/// Returns: Some(GetTabsResponse) if tabs were loaded, or None if not applicable
#[tauri::command]
pub async fn load_saved_tabs(
    state: State<'_, Mutex<VaultViewerState>>,
) -> Result<Option<GetTabsResponse>, String> {
    let mut viewer_state = state.lock().map_err(|e| format!("Failed to lock state: {}", e))?;

    // Check if auto_restore_tabs is enabled
    if !viewer_state.user_preferences.auto_restore_tabs {
        return Ok(None);
    }

    // Check if vault_root is set
    let vault_root = match &viewer_state.vault_root {
        Some(root) => root.clone(),
        None => return Ok(None),
    };

    // Load tabs from disk
    use crate::config::load_tabs;
    match load_tabs(Path::new(&vault_root)) {
        Ok(Some(tabs_state)) => {
            // Restore tabs to state
            viewer_state.open_tabs = tabs_state.open_tabs.clone();
            viewer_state.active_tab = tabs_state.active_tab.clone();

            // Return the restored tabs
            let open_tabs = viewer_state
                .open_tabs
                .iter()
                .map(|tab| TabInfo::from(tab.clone()))
                .collect();
            let active_tab = viewer_state.active_tab.clone();

            Ok(Some(GetTabsResponse {
                open_tabs,
                active_tab,
            }))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}
