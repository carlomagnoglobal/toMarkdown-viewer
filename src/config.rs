use std::path::Path;
use serde_json;
use serde::{Deserialize, Serialize};
use crate::state::{UserPreferences, TabMode, ZoomBehavior, Tab};

const CONFIG_FILENAME: &str = "vault_config.json";
const TABS_FILENAME: &str = "tabs.json";

/// Represents the state of open tabs to be persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabsState {
    pub open_tabs: Vec<Tab>,
    pub active_tab: Option<String>,
}

pub fn load_config(vault_root: &Path) -> Result<UserPreferences, String> {
    let config_path = vault_root.join(".tomarkdown").join(CONFIG_FILENAME);

    // Return defaults if config doesn't exist
    if !config_path.exists() {
        return Ok(default_preferences());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))
}

pub fn save_config(vault_root: &Path, prefs: UserPreferences) -> Result<(), String> {
    let config_dir = vault_root.join(".tomarkdown");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    let config_path = config_dir.join(CONFIG_FILENAME);
    let json = serde_json::to_string_pretty(&prefs)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&config_path, json)
        .map_err(|e| format!("Failed to write config: {}", e))
}

fn default_preferences() -> UserPreferences {
    UserPreferences {
        tab_mode: TabMode::Multi,
        theme: "system".to_string(),
        auto_restore_tabs: true,
        recycle_retention_days: 180,
        auto_save: true,
        show_toast: true,
        zoom_behavior: ZoomBehavior::ResetPerImage,
        mouse_wheel_zoom: true,
    }
}

/// Save open tabs and active tab to `.tomarkdown/tabs.json`.
///
/// Arguments:
/// - vault_root: Path to the vault root directory
/// - tabs: Slice of open tabs
/// - active_tab: ID of the currently active tab (or None)
///
/// Returns: Ok(()) on success, or an error message
pub fn save_tabs(vault_root: &Path, tabs: &[Tab], active_tab: &Option<String>) -> Result<(), String> {
    let config_dir = vault_root.join(".tomarkdown");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    let tabs_path = config_dir.join(TABS_FILENAME);
    let tabs_state = TabsState {
        open_tabs: tabs.to_vec(),
        active_tab: active_tab.clone(),
    };

    let json = serde_json::to_string_pretty(&tabs_state)
        .map_err(|e| format!("Failed to serialize tabs: {}", e))?;

    std::fs::write(&tabs_path, json)
        .map_err(|e| format!("Failed to write tabs: {}", e))
}

/// Load open tabs from `.tomarkdown/tabs.json`.
///
/// Arguments:
/// - vault_root: Path to the vault root directory
///
/// Returns: Some(TabsState) if tabs exist, None if file doesn't exist, or an error
pub fn load_tabs(vault_root: &Path) -> Result<Option<TabsState>, String> {
    let tabs_path = vault_root.join(".tomarkdown").join(TABS_FILENAME);

    // Return None if tabs file doesn't exist
    if !tabs_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&tabs_path)
        .map_err(|e| format!("Failed to read tabs: {}", e))?;

    let tabs_state: TabsState = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse tabs: {}", e))?;

    Ok(Some(tabs_state))
}

/// Update tab mode preference and save to config.
#[allow(dead_code)]
pub fn update_tab_mode(vault_root: &Path, mode: TabMode) -> Result<(), String> {
    let mut prefs = load_config(vault_root)?;
    prefs.tab_mode = mode;
    save_config(vault_root, prefs)
}

/// Update recycle bin retention days and save to config.
#[allow(dead_code)]
pub fn update_recycle_retention(vault_root: &Path, days: u32) -> Result<(), String> {
    let mut prefs = load_config(vault_root)?;
    prefs.recycle_retention_days = days;
    save_config(vault_root, prefs)
}

/// Update auto-save setting and save to config.
#[allow(dead_code)]
pub fn update_auto_save(vault_root: &Path, enabled: bool) -> Result<(), String> {
    let mut prefs = load_config(vault_root)?;
    prefs.auto_save = enabled;
    save_config(vault_root, prefs)
}

/// Update theme preference and save to config.
#[allow(dead_code)]
pub fn update_theme(vault_root: &Path, theme: String) -> Result<(), String> {
    let mut prefs = load_config(vault_root)?;
    prefs.theme = theme;
    save_config(vault_root, prefs)
}

/// Update zoom behavior and save to config.
#[allow(dead_code)]
pub fn update_zoom_behavior(vault_root: &Path, behavior: ZoomBehavior) -> Result<(), String> {
    let mut prefs = load_config(vault_root)?;
    prefs.zoom_behavior = behavior;
    save_config(vault_root, prefs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_default_config() {
        let temp_dir = TempDir::new().unwrap();
        let prefs = load_config(temp_dir.path()).unwrap();
        assert_eq!(prefs.tab_mode, TabMode::Multi);
        assert_eq!(prefs.recycle_retention_days, 180);
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let mut prefs = default_preferences();
        prefs.recycle_retention_days = 365;

        save_config(temp_dir.path(), prefs.clone()).unwrap();
        let loaded = load_config(temp_dir.path()).unwrap();

        assert_eq!(loaded.recycle_retention_days, 365);
    }

    #[test]
    fn test_load_tabs_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let result = load_tabs(temp_dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_save_and_load_tabs() {
        let temp_dir = TempDir::new().unwrap();
        let tabs = vec![
            Tab {
                id: "tab1".to_string(),
                path: "/path/to/file1.md".to_string(),
                title: "file1.md".to_string(),
                is_dirty: false,
                tab_type: "markdown".to_string(),
            },
            Tab {
                id: "tab2".to_string(),
                path: "/path/to/file2.md".to_string(),
                title: "file2.md".to_string(),
                is_dirty: true,
                tab_type: "markdown".to_string(),
            },
        ];
        let active_tab = Some("tab1".to_string());

        save_tabs(temp_dir.path(), &tabs, &active_tab).unwrap();
        let loaded = load_tabs(temp_dir.path()).unwrap();

        assert!(loaded.is_some());
        let loaded_state = loaded.unwrap();
        assert_eq!(loaded_state.open_tabs.len(), 2);
        assert_eq!(loaded_state.open_tabs[0].id, "tab1");
        assert_eq!(loaded_state.open_tabs[1].id, "tab2");
        assert_eq!(loaded_state.active_tab, Some("tab1".to_string()));
    }

    #[test]
    fn test_save_and_load_tabs_with_no_active_tab() {
        let temp_dir = TempDir::new().unwrap();
        let tabs = vec![
            Tab {
                id: "tab1".to_string(),
                path: "/path/to/file1.md".to_string(),
                title: "file1.md".to_string(),
                is_dirty: false,
                tab_type: "markdown".to_string(),
            },
        ];

        save_tabs(temp_dir.path(), &tabs, &None).unwrap();
        let loaded = load_tabs(temp_dir.path()).unwrap();

        assert!(loaded.is_some());
        let loaded_state = loaded.unwrap();
        assert_eq!(loaded_state.open_tabs.len(), 1);
        assert_eq!(loaded_state.active_tab, None);
    }
}
