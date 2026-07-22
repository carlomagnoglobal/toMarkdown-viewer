use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub id: String,
    pub path: String,
    pub title: String,
    pub is_dirty: bool,
    pub tab_type: String, // "markdown", "code", "image", "hex"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletedFile {
    pub id: String,
    pub original_path: String,
    pub vault_path: String,
    pub deleted_at: i64, // Unix timestamp
    pub file_size: u64,
}

#[derive(Debug, Clone)]
pub struct VaultViewerState {
    pub open_tabs: Vec<Tab>,
    pub active_tab: Option<String>,
    #[allow(dead_code)]
    pub preview_tab: Option<String>,
    pub tab_history: VecDeque<String>, // tab IDs for back button
    pub deleted_files: Vec<DeletedFile>,
    #[allow(dead_code)]
    pub zoom_levels: HashMap<String, f32>, // file_id -> zoom level
    pub user_preferences: UserPreferences,
    #[allow(dead_code)]
    pub vault_root: Option<String>, // Root path of the currently opened vault
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub tab_mode: TabMode,
    pub theme: String,
    pub auto_restore_tabs: bool,
    pub recycle_retention_days: u32,
    pub auto_save: bool,
    pub show_toast: bool,
    pub zoom_behavior: ZoomBehavior,
    pub mouse_wheel_zoom: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TabMode {
    Single,
    Multi,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ZoomBehavior {
    ResetPerImage,
    RememberPerImage,
    RememberGlobal,
}

impl VaultViewerState {
    pub fn new(preferences: UserPreferences) -> Self {
        VaultViewerState {
            open_tabs: Vec::new(),
            active_tab: None,
            preview_tab: None,
            tab_history: VecDeque::new(),
            deleted_files: Vec::new(),
            zoom_levels: HashMap::new(),
            user_preferences: preferences,
            vault_root: None,
        }
    }

    pub fn add_tab(&mut self, path: String, title: String, tab_type: String) -> String {
        let id = Uuid::new_v4().to_string();
        let tab = Tab {
            id: id.clone(),
            path,
            title,
            is_dirty: false,
            tab_type,
        };
        self.open_tabs.push(tab);
        self.set_active_tab(id.clone());
        id
    }

    pub fn set_active_tab(&mut self, tab_id: String) {
        if let Some(current) = &self.active_tab {
            if current != &tab_id {
                self.tab_history.push_back(current.clone());
                if self.tab_history.len() > 50 {
                    self.tab_history.pop_front();
                }
            }
        }
        self.active_tab = Some(tab_id);
    }

    pub fn close_tab(&mut self, tab_id: &str) {
        self.open_tabs.retain(|t| t.id != tab_id);
        if self.active_tab.as_ref().map_or(false, |id| id == tab_id) {
            self.active_tab = self.open_tabs.first().map(|t| t.id.clone());
        }
    }

    pub fn back(&mut self) {
        if let Some(tab_id) = self.tab_history.pop_back() {
            self.active_tab = Some(tab_id);
        }
    }

    pub fn delete_file(&mut self, path: String, vault_root: &Path) -> Result<(), String> {
        let file_path = PathBuf::from(&path);

        // Get file metadata before deletion
        let metadata = fs::metadata(&file_path)
            .map_err(|e| format!("Failed to access file: {}", e))?;
        let file_size = metadata.len();

        // Create recycle bin directory
        let recycle_bin = vault_root.join(".tomarkdown/recycle_bin");
        fs::create_dir_all(&recycle_bin)
            .map_err(|e| format!("Failed to create recycle bin: {}", e))?;

        // Generate unique ID and new location
        let file_id = Uuid::new_v4().to_string();
        let file_name = file_path
            .file_name()
            .ok_or("Invalid file name")?
            .to_str()
            .ok_or("Failed to convert file name")?
            .to_string();
        let recycled_path = recycle_bin.join(format!("{}_{}", file_id, file_name));

        // Move file to recycle bin
        fs::rename(&file_path, &recycled_path)
            .map_err(|e| format!("Failed to move file to recycle bin: {}", e))?;

        // Get current timestamp
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| format!("Failed to get timestamp: {}", e))?
            .as_secs() as i64;

        // Record in deleted_files
        let deleted_file = DeletedFile {
            id: file_id,
            original_path: path,
            vault_path: recycled_path.to_str().unwrap_or("").to_string(),
            deleted_at: timestamp,
            file_size,
        };
        self.deleted_files.push(deleted_file);

        Ok(())
    }

    pub fn restore_file(&mut self, file_id: &str, _vault_root: &Path) -> Result<(), String> {
        // Find the deleted file
        let deleted_file = self.deleted_files
            .iter()
            .find(|df| df.id == file_id)
            .ok_or("File not found in recycle bin")?
            .clone();

        let recycled_path = PathBuf::from(&deleted_file.vault_path);
        let original_path = PathBuf::from(&deleted_file.original_path);

        // Ensure parent directory exists
        if let Some(parent) = original_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        // Move file back to original location
        fs::rename(&recycled_path, &original_path)
            .map_err(|e| format!("Failed to restore file: {}", e))?;

        // Remove from deleted_files
        self.deleted_files.retain(|df| df.id != file_id);

        Ok(())
    }

    pub fn permanently_delete(&mut self, file_id: &str, _vault_root: &Path) -> Result<(), String> {
        // Find the deleted file
        let deleted_file = self.deleted_files
            .iter()
            .find(|df| df.id == file_id)
            .ok_or("File not found in recycle bin")?
            .clone();

        let recycled_path = PathBuf::from(&deleted_file.vault_path);

        // Delete the file permanently
        fs::remove_file(&recycled_path)
            .map_err(|e| format!("Failed to permanently delete file: {}", e))?;

        // Remove from deleted_files
        self.deleted_files.retain(|df| df.id != file_id);

        Ok(())
    }

    #[allow(dead_code)]
    pub fn cleanup_expired(&mut self, _vault_root: &Path, retention_days: u32) -> Result<(), String> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| format!("Failed to get current time: {}", e))?
            .as_secs() as i64;

        let retention_seconds = (retention_days as i64) * 24 * 60 * 60;

        let mut files_to_delete = Vec::new();

        // Identify expired files
        for deleted_file in &self.deleted_files {
            if now - deleted_file.deleted_at >= retention_seconds {
                files_to_delete.push(deleted_file.id.clone());
            }
        }

        // Delete expired files
        for file_id in files_to_delete {
            let deleted_file = self.deleted_files
                .iter()
                .find(|df| df.id == file_id)
                .ok_or("File not found")?
                .clone();

            let recycled_path = PathBuf::from(&deleted_file.vault_path);
            fs::remove_file(&recycled_path)
                .map_err(|e| format!("Failed to delete expired file: {}", e))?;

            self.deleted_files.retain(|df| df.id != file_id);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_create_state() {
        let prefs = UserPreferences {
            tab_mode: TabMode::Multi,
            theme: "light".to_string(),
            auto_restore_tabs: true,
            recycle_retention_days: 180,
            auto_save: true,
            show_toast: true,
            zoom_behavior: ZoomBehavior::ResetPerImage,
            mouse_wheel_zoom: true,
        };
        let state = VaultViewerState::new(prefs);
        assert_eq!(state.open_tabs.len(), 0);
        assert_eq!(state.active_tab, None);
    }

    #[test]
    fn test_add_tab() {
        let mut state = VaultViewerState::new(default_preferences());
        let id = state.add_tab(
            "/path/to/file.md".to_string(),
            "file.md".to_string(),
            "markdown".to_string(),
        );
        assert_eq!(state.open_tabs.len(), 1);
        assert_eq!(state.active_tab, Some(id));
    }

    #[test]
    fn test_close_tab() {
        let mut state = VaultViewerState::new(default_preferences());
        let id1 = state.add_tab(
            "/path/to/file1.md".to_string(),
            "file1.md".to_string(),
            "markdown".to_string(),
        );
        let _id2 = state.add_tab(
            "/path/to/file2.md".to_string(),
            "file2.md".to_string(),
            "markdown".to_string(),
        );
        state.close_tab(&id1);
        assert_eq!(state.open_tabs.len(), 1);
    }

    #[test]
    fn test_back_button() {
        let mut state = VaultViewerState::new(default_preferences());
        let id1 = state.add_tab(
            "/path/to/file1.md".to_string(),
            "file1.md".to_string(),
            "markdown".to_string(),
        );
        let _id2 = state.add_tab(
            "/path/to/file2.md".to_string(),
            "file2.md".to_string(),
            "markdown".to_string(),
        );
        state.back();
        assert_eq!(state.active_tab, Some(id1));
    }

    #[test]
    fn test_delete_file() {
        let temp_dir = TempDir::new().unwrap();
        let vault_root = temp_dir.path();

        // Create a test file
        let test_file = vault_root.join("test.md");
        let mut file = File::create(&test_file).unwrap();
        file.write_all(b"test content").unwrap();

        // Create state and delete the file
        let mut state = VaultViewerState::new(default_preferences());
        let result = state.delete_file(test_file.to_str().unwrap().to_string(), vault_root);

        assert!(result.is_ok());
        assert_eq!(state.deleted_files.len(), 1);
        assert!(!test_file.exists());
        assert!(state.deleted_files[0].original_path.ends_with("test.md"));
        assert_eq!(state.deleted_files[0].file_size, 12);
    }

    #[test]
    fn test_restore_file() {
        let temp_dir = TempDir::new().unwrap();
        let vault_root = temp_dir.path();

        // Create a test file
        let test_file = vault_root.join("test.md");
        let mut file = File::create(&test_file).unwrap();
        file.write_all(b"test content").unwrap();

        // Create state and delete the file
        let mut state = VaultViewerState::new(default_preferences());
        let delete_result = state.delete_file(test_file.to_str().unwrap().to_string(), vault_root);
        assert!(delete_result.is_ok());
        assert_eq!(state.deleted_files.len(), 1);

        // Get the file_id before restoring
        let file_id = state.deleted_files[0].id.clone();

        // Restore the file
        let restore_result = state.restore_file(&file_id, vault_root);
        assert!(restore_result.is_ok());
        assert_eq!(state.deleted_files.len(), 0);
        assert!(test_file.exists());
    }

    #[test]
    fn test_cleanup_expired_files() {
        let temp_dir = TempDir::new().unwrap();
        let vault_root = temp_dir.path();

        // Create a test file
        let test_file = vault_root.join("test.md");
        let mut file = File::create(&test_file).unwrap();
        file.write_all(b"test content").unwrap();

        // Create state and delete the file
        let mut state = VaultViewerState::new(default_preferences());
        let delete_result = state.delete_file(test_file.to_str().unwrap().to_string(), vault_root);
        assert!(delete_result.is_ok());
        assert_eq!(state.deleted_files.len(), 1);

        // Get the file ID
        let file_id = state.deleted_files[0].id.clone();
        let recycled_path = PathBuf::from(&state.deleted_files[0].vault_path);

        // Cleanup with 0 retention days should immediately delete the file
        let cleanup_result = state.cleanup_expired(vault_root, 0);
        assert!(cleanup_result.is_ok());
        assert_eq!(state.deleted_files.len(), 0);
        assert!(!recycled_path.exists(), "Recycled file should be deleted with 0 retention days");
    }

    fn default_preferences() -> UserPreferences {
        UserPreferences {
            tab_mode: TabMode::Multi,
            theme: "light".to_string(),
            auto_restore_tabs: true,
            recycle_retention_days: 180,
            auto_save: true,
            show_toast: true,
            zoom_behavior: ZoomBehavior::ResetPerImage,
            mouse_wheel_zoom: true,
        }
    }
}
