/// Basic test validating recycle bin command handlers are registered and work.
#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_recycle_commands_can_be_called() {
        // This test validates that:
        // 1. VaultViewerState can be created with recycle bin support
        // 2. Recycle bin commands operate on the state
        // 3. File deletion, restoration, and permanent deletion work

        use to_markdown_gui::state::{VaultViewerState, UserPreferences, TabMode, ZoomBehavior};

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

        let mut state = VaultViewerState::new(prefs);

        // Create a temporary directory and file for testing
        let temp_dir = TempDir::new().unwrap();
        let vault_root = temp_dir.path();
        let test_file = vault_root.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        // Test delete_file equivalent
        {
            let result = state.delete_file(test_file.to_string_lossy().to_string(), vault_root);
            assert!(result.is_ok());
            assert_eq!(state.deleted_files.len(), 1);
            assert_eq!(state.deleted_files[0].original_path, test_file.to_string_lossy().to_string());
        }

        // Test get_deleted_files equivalent
        {
            assert_eq!(state.deleted_files.len(), 1);
            let deleted = &state.deleted_files[0];
            assert!(!deleted.id.is_empty());
            assert!(deleted.file_size > 0);
        }

        // Test restore_file equivalent
        {
            let file_id = state.deleted_files[0].id.clone();
            // First recreate the file that was deleted
            let test_file_2 = vault_root.join("test2.txt");
            fs::write(&test_file_2, "test content 2").unwrap();
            let _ = state.delete_file(test_file_2.to_string_lossy().to_string(), vault_root);

            // Restore the first file
            let result = state.restore_file(&file_id, vault_root);
            assert!(result.is_ok());
            // File should be restored to original location
            assert!(test_file.exists());
            assert_eq!(state.deleted_files.len(), 1); // One file remains in recycle bin
        }

        // Test permanently_delete equivalent
        {
            if !state.deleted_files.is_empty() {
                let file_id = state.deleted_files[0].id.clone();
                let result = state.permanently_delete(&file_id, vault_root);
                assert!(result.is_ok());
                assert_eq!(state.deleted_files.len(), 0);
            }
        }
    }

    #[test]
    fn test_empty_recycle_bin() {
        use to_markdown_gui::state::{VaultViewerState, UserPreferences, TabMode, ZoomBehavior};

        let prefs = UserPreferences {
            tab_mode: TabMode::Single,
            theme: "dark".to_string(),
            auto_restore_tabs: false,
            recycle_retention_days: 90,
            auto_save: false,
            show_toast: false,
            zoom_behavior: ZoomBehavior::RememberPerImage,
            mouse_wheel_zoom: false,
        };

        let state = VaultViewerState::new(prefs);

        // Verify initial state has empty recycle bin
        assert_eq!(state.deleted_files.len(), 0);
    }

    #[test]
    fn test_cleanup_expired_files() {
        use to_markdown_gui::state::{VaultViewerState, UserPreferences, TabMode, ZoomBehavior};

        let prefs = UserPreferences {
            tab_mode: TabMode::Multi,
            theme: "light".to_string(),
            auto_restore_tabs: true,
            recycle_retention_days: 0, // No retention
            auto_save: true,
            show_toast: true,
            zoom_behavior: ZoomBehavior::ResetPerImage,
            mouse_wheel_zoom: true,
        };

        let mut state = VaultViewerState::new(prefs);

        // Create a temporary directory and file for testing
        let temp_dir = TempDir::new().unwrap();
        let vault_root = temp_dir.path();
        let test_file = vault_root.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        // Delete a file
        let _ = state.delete_file(test_file.to_string_lossy().to_string(), vault_root);
        assert_eq!(state.deleted_files.len(), 1);

        // Cleanup expired files with 0 retention days
        let result = state.cleanup_expired(vault_root, 0);
        assert!(result.is_ok());
        // File should be cleaned up
        assert_eq!(state.deleted_files.len(), 0);
    }
}
