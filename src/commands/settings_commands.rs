use crate::config;
use crate::state::UserPreferences;
use std::path::Path;

/// Retrieve current user preferences for the specified vault.
#[tauri::command]
pub async fn get_preferences(vault_root: String) -> Result<UserPreferences, String> {
    config::load_config(Path::new(&vault_root))
}

/// Update user preferences for the specified vault and return the updated preferences.
#[tauri::command]
pub async fn update_preferences(
    vault_root: String,
    preferences: UserPreferences,
) -> Result<UserPreferences, String> {
    config::save_config(Path::new(&vault_root), preferences.clone())?;
    Ok(preferences)
}
