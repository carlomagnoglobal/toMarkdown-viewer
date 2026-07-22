/// Basic test validating tab command handlers are registered and work.
#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    // We import from the lib to test the state management
    #[test]
    fn test_tab_commands_can_be_called() {
        // This test validates that:
        // 1. VaultViewerState can be created
        // 2. Tab commands operate on the state
        // 3. Basic tab lifecycle works

        // Simulate the app's state initialization
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

        let state = Mutex::new(VaultViewerState::new(prefs));

        // Test add_tab equivalent
        {
            let mut viewer_state = state.lock().unwrap();
            let tab_id = viewer_state.add_tab(
                "/path/to/file.md".to_string(),
                "file.md".to_string(),
                "markdown".to_string(),
            );
            assert!(!tab_id.is_empty());
            assert_eq!(viewer_state.open_tabs.len(), 1);
            assert_eq!(viewer_state.active_tab, Some(tab_id.clone()));
        }

        // Test set_active_tab equivalent
        {
            let mut viewer_state = state.lock().unwrap();
            let tab_id_2 = viewer_state.add_tab(
                "/path/to/file2.md".to_string(),
                "file2.md".to_string(),
                "markdown".to_string(),
            );
            viewer_state.set_active_tab(tab_id_2.clone());
            assert_eq!(viewer_state.active_tab, Some(tab_id_2));
        }

        // Test get_tabs equivalent
        {
            let viewer_state = state.lock().unwrap();
            assert_eq!(viewer_state.open_tabs.len(), 2);
            assert!(viewer_state.active_tab.is_some());
        }

        // Test back_button equivalent
        {
            let mut viewer_state = state.lock().unwrap();
            let first_tab_id = viewer_state.open_tabs[0].id.clone();
            viewer_state.back();
            assert_eq!(viewer_state.active_tab, Some(first_tab_id));
        }

        // Test close_tab equivalent
        {
            let mut viewer_state = state.lock().unwrap();
            let tab_to_close = viewer_state.open_tabs[1].id.clone();
            viewer_state.close_tab(&tab_to_close);
            assert_eq!(viewer_state.open_tabs.len(), 1);
        }
    }

    #[test]
    fn test_empty_state() {
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

        assert_eq!(state.open_tabs.len(), 0);
        assert_eq!(state.active_tab, None);
        assert_eq!(state.tab_history.len(), 0);
    }
}
