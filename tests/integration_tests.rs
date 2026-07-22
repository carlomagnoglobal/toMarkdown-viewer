//! Comprehensive integration test suite verifying all features work together.
//!
//! This suite tests:
//! 1. Tab workflow (create → switch → close)
//! 2. Recycle bin workflow (delete → restore → permanent delete)
//! 3. Copy operations for all 6 formats (base64, markdown, hex, SHA256, MD5, CRC)
//! 4. Image zoom workflow (zoom in/out, fit, pan)
//! 5. Settings persistence (save/load preferences)
//! 6. Tab persistence (save/restore across sessions)
//!
//! Each test is independent and uses TempDir for isolation.

use std::fs;
use tempfile::TempDir;

// Import library modules
use to_markdown_gui::clipboard_ops;
use to_markdown_gui::config;
use to_markdown_gui::image_zoom::ZoomCalculator;
use to_markdown_gui::state::{UserPreferences, TabMode, ZoomBehavior, VaultViewerState};

// Base64 imports
use base64::Engine;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Approximate floating point equality with epsilon tolerance
fn approx_eq(left: f32, right: f32, epsilon: f32) -> bool {
    (left - right).abs() < epsilon
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

fn create_test_vault() -> TempDir {
    let vault = TempDir::new().expect("Failed to create temp vault");
    let vault_path = vault.path();

    // Create directory structure
    fs::create_dir_all(vault_path.join(".tomarkdown")).expect("Failed to create .tomarkdown");
    fs::create_dir_all(vault_path.join("documents")).expect("Failed to create documents");

    // Create test files
    fs::write(
        vault_path.join("documents/test1.md"),
        "# Test Document 1\n\nContent here.",
    )
    .expect("Failed to write test1.md");

    fs::write(
        vault_path.join("documents/test2.md"),
        "# Test Document 2\n\nMore content.",
    )
    .expect("Failed to write test2.md");

    fs::write(vault_path.join("documents/hello.rs"), "fn main() { println!(\"Hello\"); }")
        .expect("Failed to write hello.rs");

    vault
}

// ============================================================================
// TEST 1: TAB WORKFLOW (create → switch → close)
// ============================================================================

#[test]
fn test_tab_workflow_create_switch_close() {
    let mut state = VaultViewerState::new(default_preferences());

    // Step 1: Create first tab
    let tab1_id = state.add_tab(
        "/vault/documents/test1.md".to_string(),
        "test1.md".to_string(),
        "markdown".to_string(),
    );
    assert_eq!(state.open_tabs.len(), 1);
    assert_eq!(state.active_tab, Some(tab1_id.clone()));

    // Step 2: Create second tab
    let tab2_id = state.add_tab(
        "/vault/documents/test2.md".to_string(),
        "test2.md".to_string(),
        "markdown".to_string(),
    );
    assert_eq!(state.open_tabs.len(), 2);
    assert_eq!(state.active_tab, Some(tab2_id.clone()));

    // Step 3: Create third tab
    let tab3_id = state.add_tab(
        "/vault/documents/hello.rs".to_string(),
        "hello.rs".to_string(),
        "code".to_string(),
    );
    assert_eq!(state.open_tabs.len(), 3);
    assert_eq!(state.active_tab, Some(tab3_id.clone()));

    // Step 4: Switch to first tab
    state.set_active_tab(tab1_id.clone());
    assert_eq!(state.active_tab, Some(tab1_id.clone()));
    assert_eq!(state.tab_history.len(), 3); // Should have history: tab1, tab2, tab3

    // Step 5: Switch to second tab
    state.set_active_tab(tab2_id.clone());
    assert_eq!(state.active_tab, Some(tab2_id.clone()));

    // Step 6: Close first tab
    state.close_tab(&tab1_id);
    assert_eq!(state.open_tabs.len(), 2);
    assert!(!state.open_tabs.iter().any(|t| t.id == tab1_id));

    // Step 7: Close second tab (active tab)
    state.close_tab(&tab2_id);
    assert_eq!(state.open_tabs.len(), 1);
    assert_eq!(state.active_tab, Some(tab3_id.clone()));

    // Step 8: Close all tabs
    state.close_tab(&tab3_id);
    assert_eq!(state.open_tabs.len(), 0);
    assert_eq!(state.active_tab, None);
}

#[test]
fn test_tab_workflow_back_button() {
    let mut state = VaultViewerState::new(default_preferences());

    // Create three tabs
    let tab1_id = state.add_tab(
        "/vault/file1.md".to_string(),
        "file1.md".to_string(),
        "markdown".to_string(),
    );
    let tab2_id = state.add_tab(
        "/vault/file2.md".to_string(),
        "file2.md".to_string(),
        "markdown".to_string(),
    );
    let tab3_id = state.add_tab(
        "/vault/file3.md".to_string(),
        "file3.md".to_string(),
        "markdown".to_string(),
    );

    // Navigate: 1 → 2 → 3 → back should go to 2
    assert_eq!(state.active_tab, Some(tab3_id.clone()));
    state.back();
    assert_eq!(state.active_tab, Some(tab2_id.clone()));

    // Back again should go to 1
    state.back();
    assert_eq!(state.active_tab, Some(tab1_id.clone()));

    // Back with no history should stay at 1
    state.back();
    assert_eq!(state.active_tab, Some(tab1_id.clone()));
}

#[test]
fn test_tab_workflow_multiple_tabs_same_file() {
    let mut state = VaultViewerState::new(default_preferences());

    // Open same file twice
    let tab1_id = state.add_tab(
        "/vault/test.md".to_string(),
        "test.md".to_string(),
        "markdown".to_string(),
    );
    let tab2_id = state.add_tab(
        "/vault/test.md".to_string(),
        "test.md".to_string(),
        "markdown".to_string(),
    );

    // Should be different tab IDs
    assert_ne!(tab1_id, tab2_id);
    assert_eq!(state.open_tabs.len(), 2);

    // Close one tab
    state.close_tab(&tab1_id);
    assert_eq!(state.open_tabs.len(), 1);

    // Other tab should still exist
    assert_eq!(state.active_tab, Some(tab2_id));
}

// ============================================================================
// TEST 2: RECYCLE BIN WORKFLOW (delete → restore → permanent delete)
// ============================================================================

#[test]
fn test_recycle_bin_workflow_delete_restore() {
    let vault = create_test_vault();
    let vault_path = vault.path();
    let test_file = vault_path.join("documents/test1.md");

    let mut state = VaultViewerState::new(default_preferences());

    // Step 1: Verify file exists
    assert!(test_file.exists());

    // Step 2: Delete file to recycle bin
    let result = state.delete_file(test_file.to_string_lossy().to_string(), vault_path);
    assert!(result.is_ok());
    assert_eq!(state.deleted_files.len(), 1);
    assert!(!test_file.exists());

    // Step 3: Verify deletion metadata
    let deleted = &state.deleted_files[0];
    assert_eq!(deleted.original_path, test_file.to_string_lossy().to_string());
    assert!(deleted.file_size > 0);

    // Step 4: Restore file from recycle bin
    let file_id = deleted.id.clone();
    let result = state.restore_file(&file_id, vault_path);
    assert!(result.is_ok());
    assert_eq!(state.deleted_files.len(), 0);
    assert!(test_file.exists());

    // Step 5: Verify content is preserved
    let content = fs::read_to_string(&test_file).expect("Failed to read restored file");
    assert!(content.contains("Test Document 1"));
}

#[test]
fn test_recycle_bin_workflow_delete_permanent() {
    let vault = create_test_vault();
    let vault_path = vault.path();
    let test_file = vault_path.join("documents/test1.md");

    let mut state = VaultViewerState::new(default_preferences());

    // Delete file
    state
        .delete_file(test_file.to_string_lossy().to_string(), vault_path)
        .expect("Delete should succeed");
    assert_eq!(state.deleted_files.len(), 1);

    // Permanently delete
    let file_id = state.deleted_files[0].id.clone();
    let result = state.permanently_delete(&file_id, vault_path);
    assert!(result.is_ok());
    assert_eq!(state.deleted_files.len(), 0);
}

#[test]
fn test_recycle_bin_workflow_multiple_files() {
    let vault = create_test_vault();
    let vault_path = vault.path();
    let test1 = vault_path.join("documents/test1.md");
    let test2 = vault_path.join("documents/test2.md");

    let mut state = VaultViewerState::new(default_preferences());

    // Delete two files
    state
        .delete_file(test1.to_string_lossy().to_string(), vault_path)
        .expect("Delete test1");
    state
        .delete_file(test2.to_string_lossy().to_string(), vault_path)
        .expect("Delete test2");
    assert_eq!(state.deleted_files.len(), 2);

    // Restore first file
    let id1 = state.deleted_files[0].id.clone();
    state.restore_file(&id1, vault_path).expect("Restore test1");
    assert_eq!(state.deleted_files.len(), 1);
    assert!(test1.exists());
    assert!(!test2.exists());

    // Restore second file
    let id2 = state.deleted_files[0].id.clone();
    state.restore_file(&id2, vault_path).expect("Restore test2");
    assert_eq!(state.deleted_files.len(), 0);
    assert!(test1.exists());
    assert!(test2.exists());
}

#[test]
fn test_recycle_bin_workflow_cleanup_expired() {
    let vault = create_test_vault();
    let vault_path = vault.path();
    let test_file = vault_path.join("documents/test1.md");

    let mut state = VaultViewerState::new(default_preferences());

    // Delete file
    state
        .delete_file(test_file.to_string_lossy().to_string(), vault_path)
        .expect("Delete should succeed");
    assert_eq!(state.deleted_files.len(), 1);

    // Cleanup with 0 retention days should delete immediately
    let result = state.cleanup_expired(vault_path, 0);
    assert!(result.is_ok());
    assert_eq!(state.deleted_files.len(), 0);
}

// ============================================================================
// TEST 3: COPY OPERATIONS (all 6 formats)
// ============================================================================

#[test]
fn test_copy_format_base64() {
    let vault = create_test_vault();
    let test_file = vault.path().join("documents/test1.md");

    let result = clipboard_ops::copy_as_base64(&test_file);
    assert!(result.is_ok());

    let b64 = result.unwrap();
    assert!(!b64.is_empty());

    // Decode and verify
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&b64)
        .expect("Decode should succeed");
    let text = String::from_utf8(decoded).expect("Should be valid UTF-8");
    assert!(text.contains("Test Document 1"));
}

#[test]
fn test_copy_format_markdown() {
    let vault = create_test_vault();
    let test_file = vault.path().join("documents/test1.md");

    let result = clipboard_ops::copy_as_markdown(&test_file, "markdown");
    assert!(result.is_ok());

    let md = result.unwrap();
    assert!(md.contains("Test Document 1"));
}

#[test]
fn test_copy_format_markdown_code_file() {
    let vault = create_test_vault();
    let code_file = vault.path().join("documents/hello.rs");

    let result = clipboard_ops::copy_as_markdown(&code_file, "code");
    assert!(result.is_ok());

    let md = result.unwrap();
    assert!(md.contains("```rust"));
    assert!(md.contains("fn main()"));
}

#[test]
fn test_copy_format_hex() {
    let vault = create_test_vault();
    let test_file = vault.path().join("documents/test1.md");

    let result = clipboard_ops::copy_as_hex(&test_file);
    assert!(result.is_ok());

    let hex = result.unwrap();
    assert!(!hex.is_empty());
    // Hex should only contain valid hex characters
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_copy_format_sha256() {
    let vault = create_test_vault();
    let test_file = vault.path().join("documents/test1.md");

    let result = clipboard_ops::copy_sha256(&test_file);
    assert!(result.is_ok());

    let hash = result.unwrap();
    // SHA256 produces 64 character hex string
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_copy_format_md5() {
    let vault = create_test_vault();
    let test_file = vault.path().join("documents/test1.md");

    let result = clipboard_ops::copy_md5(&test_file);
    assert!(result.is_ok());

    let hash = result.unwrap();
    // MD5 produces 32 character hex string
    assert_eq!(hash.len(), 32);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_copy_format_crc() {
    let vault = create_test_vault();
    let test_file = vault.path().join("documents/test1.md");

    let result = clipboard_ops::copy_crc(&test_file);
    assert!(result.is_ok());

    let hash = result.unwrap();
    // CRC produces 8 character hex string
    assert_eq!(hash.len(), 8);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_copy_operations_all_formats_same_file() {
    let vault = create_test_vault();
    let test_file = vault.path().join("documents/test1.md");

    // Test all formats on the same file
    let base64 = clipboard_ops::copy_as_base64(&test_file).expect("Base64 failed");
    let markdown = clipboard_ops::copy_as_markdown(&test_file, "markdown").expect("Markdown failed");
    let hex = clipboard_ops::copy_as_hex(&test_file).expect("Hex failed");
    let sha256 = clipboard_ops::copy_sha256(&test_file).expect("SHA256 failed");
    let md5 = clipboard_ops::copy_md5(&test_file).expect("MD5 failed");
    let crc = clipboard_ops::copy_crc(&test_file).expect("CRC failed");

    // All should produce non-empty results
    assert!(!base64.is_empty());
    assert!(!markdown.is_empty());
    assert!(!hex.is_empty());
    assert_eq!(sha256.len(), 64);
    assert_eq!(md5.len(), 32);
    assert_eq!(crc.len(), 8);

    // Hashes should be consistent
    let sha256_again = clipboard_ops::copy_sha256(&test_file).expect("SHA256 again failed");
    assert_eq!(sha256, sha256_again);
}

// ============================================================================
// TEST 4: IMAGE ZOOM WORKFLOW
// ============================================================================

#[test]
fn test_image_zoom_workflow_zoom_in_out() {
    let mut zoom = ZoomCalculator::new(800, 600, 400, 300);

    // Initial zoom
    assert_eq!(zoom.get_current_zoom(), 1.0);

    // Zoom in 5 times
    for _ in 0..5 {
        zoom.zoom_in();
    }
    assert!(approx_eq(zoom.get_current_zoom(), 1.5, 0.0001), "Expected 1.5, got {}", zoom.get_current_zoom());

    // Zoom out 3 times
    for _ in 0..3 {
        zoom.zoom_out();
    }
    assert!(approx_eq(zoom.get_current_zoom(), 1.2, 0.0001), "Expected 1.2, got {}", zoom.get_current_zoom());

    // Zoom out to minimum
    for _ in 0..20 {
        zoom.zoom_out();
    }
    assert_eq!(zoom.get_current_zoom(), 1.0);
}

#[test]
fn test_image_zoom_workflow_fit_to_window() {
    let mut zoom = ZoomCalculator::new(1000, 800, 500, 400);

    // Zoom in first
    for _ in 0..30 {
        zoom.zoom_in();
    }
    assert!(zoom.get_current_zoom() > 1.0);

    // Fit to window should scale appropriately
    zoom.fit_to_window();

    // Should fit image in viewport
    let (width, height) = zoom.get_display_dimensions();
    assert!(width <= 500);
    assert!(height <= 400);

    // Pan should be reset
    let (pan_x, pan_y) = zoom.get_pan_offset();
    assert_eq!(pan_x, 0);
    assert_eq!(pan_y, 0);
}

#[test]
fn test_image_zoom_workflow_pan() {
    let mut zoom = ZoomCalculator::new(100, 100, 50, 50);

    // Zoom in to enable panning
    zoom.current_zoom = 3.0; // 300x300 displayed in 50x50 viewport

    // Pan right
    zoom.pan(50, 0);
    let (pan_x, pan_y) = zoom.get_pan_offset();
    assert!(pan_x > 0);
    assert_eq!(pan_y, 0);

    // Pan down
    zoom.pan(0, 50);
    let (pan_x, pan_y) = zoom.get_pan_offset();
    assert!(pan_y > 0);

    // Pan with boundary checking
    zoom.pan(10000, 10000); // Try to pan way beyond bounds
    let (pan_x, pan_y) = zoom.get_pan_offset();
    assert!(pan_x <= 250); // 300 - 50
    assert!(pan_y <= 250);
}

#[test]
fn test_image_zoom_workflow_reset() {
    let mut zoom = ZoomCalculator::new(800, 600, 400, 300);

    // Zoom and pan
    zoom.zoom_in();
    zoom.zoom_in();
    zoom.zoom_in();
    zoom.pan(50, 50);

    assert!(zoom.get_current_zoom() > 1.0);
    let (pan_x, pan_y) = zoom.get_pan_offset();
    assert!(pan_x > 0 || pan_y > 0);

    // Reset
    zoom.reset_zoom();

    // Should be back to defaults
    assert_eq!(zoom.get_current_zoom(), 1.0);
    let (pan_x, pan_y) = zoom.get_pan_offset();
    assert_eq!(pan_x, 0);
    assert_eq!(pan_y, 0);
}

#[test]
fn test_image_zoom_workflow_mouse_wheel() {
    let mut zoom = ZoomCalculator::new(800, 600, 400, 300);

    // Positive delta (scroll up) = zoom in
    zoom.mouse_wheel_zoom(1);
    assert!(approx_eq(zoom.get_current_zoom(), 1.1, 0.0001), "Expected 1.1, got {}", zoom.get_current_zoom());

    // Negative delta (scroll down) = zoom out
    zoom.mouse_wheel_zoom(-1);
    assert!(approx_eq(zoom.get_current_zoom(), 1.0, 0.0001), "Expected 1.0, got {}", zoom.get_current_zoom());

    // Multiple wheel events (each call is a single step, so 5 positive deltas = zoom in 5 times)
    for _ in 0..5 {
        zoom.mouse_wheel_zoom(1);
    }
    assert!(approx_eq(zoom.get_current_zoom(), 1.5, 0.0001), "Expected 1.5, got {}", zoom.get_current_zoom());
}

#[test]
fn test_image_zoom_workflow_complete() {
    let mut zoom = ZoomCalculator::new(1200, 900, 600, 450);

    // Start at 1.0x
    assert_eq!(zoom.get_current_zoom(), 1.0);

    // Zoom in via mouse wheel
    zoom.mouse_wheel_zoom(10);
    let zoomed = zoom.get_current_zoom();
    assert!(zoomed > 1.0);

    // Get display dimensions
    let (display_w, display_h) = zoom.get_display_dimensions();
    assert_eq!(display_w as f32, 1200.0 * zoomed);
    assert_eq!(display_h as f32, 900.0 * zoomed);

    // Pan to new location
    zoom.pan(100, 100);
    let (pan_x, pan_y) = zoom.get_pan_offset();
    assert!(pan_x > 0);
    assert!(pan_y > 0);

    // Fit to window
    zoom.fit_to_window();
    assert_eq!(zoom.get_current_zoom(), 0.5); // 1200 / 600 = 2.0, 900 / 450 = 2.0, min is 0.5

    // Pan should be reset after fit
    let (pan_x, pan_y) = zoom.get_pan_offset();
    assert_eq!(pan_x, 0);
    assert_eq!(pan_y, 0);
}

// ============================================================================
// TEST 5: SETTINGS PERSISTENCE
// ============================================================================

#[test]
fn test_settings_persistence_save_load_config() {
    let vault = create_test_vault();
    let vault_path = vault.path();

    let mut prefs = default_preferences();
    prefs.tab_mode = TabMode::Single;
    prefs.theme = "dark".to_string();
    prefs.recycle_retention_days = 365;
    prefs.auto_save = false;

    // Save config
    let result = config::save_config(vault_path, prefs.clone());
    assert!(result.is_ok());

    // Load config
    let loaded = config::load_config(vault_path).expect("Load should succeed");

    // Verify all settings were persisted
    assert_eq!(loaded.tab_mode, TabMode::Single);
    assert_eq!(loaded.theme, "dark");
    assert_eq!(loaded.recycle_retention_days, 365);
    assert!(!loaded.auto_save);
}

#[test]
fn test_settings_persistence_load_defaults() {
    let vault = TempDir::new().expect("Failed to create temp vault");
    let vault_path = vault.path();

    // Create .tomarkdown directory but no config file
    fs::create_dir_all(vault_path.join(".tomarkdown"))
        .expect("Failed to create .tomarkdown");

    // Loading should return defaults
    let loaded = config::load_config(vault_path).expect("Load should succeed");

    // Check defaults
    assert_eq!(loaded.tab_mode, TabMode::Multi);
    assert_eq!(loaded.theme, "system");
    assert_eq!(loaded.recycle_retention_days, 180);
    assert!(loaded.auto_save);
}

#[test]
fn test_settings_persistence_update_individual_settings() {
    let vault = create_test_vault();
    let vault_path = vault.path();

    // Update individual settings
    config::update_tab_mode(vault_path, TabMode::Single).expect("Update tab mode");
    config::update_theme(vault_path, "dark".to_string()).expect("Update theme");
    config::update_recycle_retention(vault_path, 365).expect("Update retention");

    // Load and verify
    let loaded = config::load_config(vault_path).expect("Load should succeed");
    assert_eq!(loaded.tab_mode, TabMode::Single);
    assert_eq!(loaded.theme, "dark");
    assert_eq!(loaded.recycle_retention_days, 365);
}

#[test]
fn test_settings_persistence_zoom_behavior() {
    let vault = create_test_vault();
    let vault_path = vault.path();

    let behaviors = vec![
        ZoomBehavior::ResetPerImage,
        ZoomBehavior::RememberPerImage,
        ZoomBehavior::RememberGlobal,
    ];

    for behavior in behaviors {
        config::update_zoom_behavior(vault_path, behavior.clone())
            .expect("Update zoom behavior");

        let loaded = config::load_config(vault_path).expect("Load should succeed");
        assert_eq!(loaded.zoom_behavior, behavior);
    }
}

// ============================================================================
// TEST 6: TAB PERSISTENCE (save/restore across sessions)
// ============================================================================

#[test]
fn test_tab_persistence_save_and_load() {
    let vault = create_test_vault();
    let vault_path = vault.path();

    // Create tabs
    let tabs = vec![
        to_markdown_gui::state::Tab {
            id: "tab1".to_string(),
            path: "/vault/file1.md".to_string(),
            title: "file1.md".to_string(),
            is_dirty: false,
            tab_type: "markdown".to_string(),
        },
        to_markdown_gui::state::Tab {
            id: "tab2".to_string(),
            path: "/vault/file2.md".to_string(),
            title: "file2.md".to_string(),
            is_dirty: true,
            tab_type: "markdown".to_string(),
        },
    ];
    let active_tab = Some("tab1".to_string());

    // Save tabs
    config::save_tabs(vault_path, &tabs, &active_tab).expect("Save tabs should succeed");

    // Load tabs
    let loaded = config::load_tabs(vault_path)
        .expect("Load tabs should succeed")
        .expect("Tabs should exist");

    // Verify all data persisted
    assert_eq!(loaded.open_tabs.len(), 2);
    assert_eq!(loaded.open_tabs[0].id, "tab1");
    assert_eq!(loaded.open_tabs[1].id, "tab2");
    assert_eq!(loaded.open_tabs[0].is_dirty, false);
    assert_eq!(loaded.open_tabs[1].is_dirty, true);
    assert_eq!(loaded.active_tab, Some("tab1".to_string()));
}

#[test]
fn test_tab_persistence_empty_tabs() {
    let vault = TempDir::new().expect("Failed to create temp vault");
    let vault_path = vault.path();

    fs::create_dir_all(vault_path.join(".tomarkdown")).expect("Create .tomarkdown");

    // Load tabs when none exist
    let result = config::load_tabs(vault_path).expect("Load should succeed");
    assert!(result.is_none());
}

#[test]
fn test_tab_persistence_no_active_tab() {
    let vault = create_test_vault();
    let vault_path = vault.path();

    let tabs = vec![to_markdown_gui::state::Tab {
        id: "tab1".to_string(),
        path: "/vault/file1.md".to_string(),
        title: "file1.md".to_string(),
        is_dirty: false,
        tab_type: "markdown".to_string(),
    }];

    // Save with no active tab
    config::save_tabs(vault_path, &tabs, &None).expect("Save should succeed");

    // Load and verify
    let loaded = config::load_tabs(vault_path)
        .expect("Load should succeed")
        .expect("Should have tabs");

    assert_eq!(loaded.open_tabs.len(), 1);
    assert_eq!(loaded.active_tab, None);
}

#[test]
fn test_tab_persistence_multiple_file_types() {
    let vault = create_test_vault();
    let vault_path = vault.path();

    let tabs = vec![
        to_markdown_gui::state::Tab {
            id: "md1".to_string(),
            path: "/vault/doc.md".to_string(),
            title: "doc.md".to_string(),
            is_dirty: false,
            tab_type: "markdown".to_string(),
        },
        to_markdown_gui::state::Tab {
            id: "code1".to_string(),
            path: "/vault/main.rs".to_string(),
            title: "main.rs".to_string(),
            is_dirty: true,
            tab_type: "code".to_string(),
        },
        to_markdown_gui::state::Tab {
            id: "img1".to_string(),
            path: "/vault/image.png".to_string(),
            title: "image.png".to_string(),
            is_dirty: false,
            tab_type: "image".to_string(),
        },
        to_markdown_gui::state::Tab {
            id: "hex1".to_string(),
            path: "/vault/data.bin".to_string(),
            title: "data.bin".to_string(),
            is_dirty: false,
            tab_type: "hex".to_string(),
        },
    ];
    let active_tab = Some("code1".to_string());

    // Save tabs
    config::save_tabs(vault_path, &tabs, &active_tab).expect("Save should succeed");

    // Load and verify
    let loaded = config::load_tabs(vault_path)
        .expect("Load should succeed")
        .expect("Should have tabs");

    assert_eq!(loaded.open_tabs.len(), 4);

    // Verify each tab type
    assert_eq!(loaded.open_tabs[0].tab_type, "markdown");
    assert_eq!(loaded.open_tabs[1].tab_type, "code");
    assert_eq!(loaded.open_tabs[2].tab_type, "image");
    assert_eq!(loaded.open_tabs[3].tab_type, "hex");

    // Verify active tab
    assert_eq!(loaded.active_tab, Some("code1".to_string()));
}

// ============================================================================
// INTEGRATION TEST: Complete workflow combining multiple features
// ============================================================================

#[test]
fn test_integration_complete_workflow() {
    let vault = create_test_vault();
    let vault_path = vault.path();
    let test_file = vault.path().join("documents/test1.md");

    // ===== PART 1: Tab Management =====
    let mut state = VaultViewerState::new(default_preferences());

    let tab1 = state.add_tab(
        test_file.to_string_lossy().to_string(),
        "test1.md".to_string(),
        "markdown".to_string(),
    );
    let tab2 = state.add_tab(
        vault_path.join("documents/test2.md").to_string_lossy().to_string(),
        "test2.md".to_string(),
        "markdown".to_string(),
    );

    assert_eq!(state.open_tabs.len(), 2);

    // ===== PART 2: File Operations (Copy) =====
    let hash = clipboard_ops::copy_sha256(&test_file).expect("Hash failed");
    assert_eq!(hash.len(), 64);

    // ===== PART 3: Recycle Bin =====
    state
        .delete_file(test_file.to_string_lossy().to_string(), vault_path)
        .expect("Delete failed");
    assert_eq!(state.deleted_files.len(), 1);
    assert!(!test_file.exists());

    // ===== PART 4: Restore =====
    let file_id = state.deleted_files[0].id.clone();
    state.restore_file(&file_id, vault_path).expect("Restore failed");
    assert_eq!(state.deleted_files.len(), 0);
    assert!(test_file.exists());

    // ===== PART 5: Settings Persistence =====
    let mut prefs = default_preferences();
    prefs.theme = "dark".to_string();
    config::save_config(vault_path, prefs).expect("Save config failed");

    let loaded_prefs = config::load_config(vault_path).expect("Load config failed");
    assert_eq!(loaded_prefs.theme, "dark");

    // ===== PART 6: Tab Persistence =====
    config::save_tabs(vault_path, &state.open_tabs, &state.active_tab)
        .expect("Save tabs failed");

    let loaded_tabs = config::load_tabs(vault_path)
        .expect("Load tabs failed")
        .expect("Tabs should exist");
    assert_eq!(loaded_tabs.open_tabs.len(), 2);

    // ===== PART 7: Zoom Simulation =====
    let mut zoom = ZoomCalculator::new(800, 600, 400, 300);
    zoom.zoom_in();
    zoom.zoom_in();
    zoom.pan(50, 50);
    assert!(zoom.get_current_zoom() > 1.0);

    // All systems working together
    println!("Integration test passed: All features working together");
}

// Import base64 for decoding tests
use base64;
