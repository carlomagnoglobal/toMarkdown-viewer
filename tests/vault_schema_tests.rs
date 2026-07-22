use tempfile::TempDir;
use to_markdown_gui::vault::init_vault_db;

#[test]
fn test_init_vault_db() {
    // Create a temporary directory for the vault
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let vault_path = temp_dir.path();

    // Initialize the vault database
    let vault_db = init_vault_db(vault_path).expect("Failed to initialize vault database");

    // Verify the database file exists at .tomarkdown/vault.db
    let expected_db_path = vault_path.join(".tomarkdown").join("vault.db");
    assert!(
        expected_db_path.exists(),
        "Database file should exist at {:?}",
        expected_db_path
    );

    // Verify vault_root is set correctly
    assert_eq!(
        vault_db.vault_root, vault_path,
        "vault_root should match the input path"
    );
}

#[test]
fn test_vault_schema_tables_exist() {
    // Create a temporary directory for the vault
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let vault_path = temp_dir.path();

    // Initialize the vault database
    let vault_db = init_vault_db(vault_path).expect("Failed to initialize vault database");

    // Query sqlite_master to verify all tables exist
    let expected_tables = vec!["files", "file_links", "word_graph", "index_state", "recent_files"];

    for table_name in expected_tables {
        let mut stmt = vault_db
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name=?1")
            .expect("Failed to prepare statement");

        let table_exists = stmt
            .exists([table_name])
            .expect("Failed to execute statement");

        assert!(
            table_exists,
            "Table '{}' should exist in the database",
            table_name
        );
    }

    // Verify the virtual FTS table exists
    let mut stmt = vault_db
        .conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='files_fts'")
        .expect("Failed to prepare statement");

    let fts_exists = stmt
        .exists([])
        .expect("Failed to execute statement");

    assert!(fts_exists, "Virtual FTS table 'files_fts' should exist in the database");
}
