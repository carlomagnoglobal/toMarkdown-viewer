use tempfile::TempDir;
use to_markdown_gui::vault::init_vault_db;

#[test]
fn test_search_files_basic() {
    // Create a temporary directory for the test vault
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let vault_path = temp_dir.path();

    // Initialize the vault database
    let vault_db = init_vault_db(vault_path).expect("Failed to initialize vault database");

    // Insert some test data into the files table
    vault_db
        .conn
        .execute(
            "INSERT INTO files (path, name, extension, file_type, language, size_bytes, is_indexed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "notes/project.md",
                "project.md",
                "md",
                "markdown",
                "markdown",
                1024i64,
                1i32
            ],
        )
        .expect("Failed to insert project file");

    vault_db
        .conn
        .execute(
            "INSERT INTO files (path, name, extension, file_type, language, size_bytes, is_indexed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "docs/readme.md",
                "readme.md",
                "md",
                "markdown",
                "markdown",
                2048i64,
                1i32
            ],
        )
        .expect("Failed to insert readme file");

    vault_db
        .conn
        .execute(
            "INSERT INTO files (path, name, extension, file_type, language, size_bytes, is_indexed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "src/main.rs",
                "main.rs",
                "rs",
                "code",
                "rust",
                4096i64,
                1i32
            ],
        )
        .expect("Failed to insert rust file");

    // Insert into FTS table
    vault_db
        .conn
        .execute(
            "INSERT INTO files_fts (path, name, content, language) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["notes/project.md", "project.md", "Project management notes", "markdown"],
        )
        .expect("Failed to insert into FTS");

    vault_db
        .conn
        .execute(
            "INSERT INTO files_fts (path, name, content, language) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["docs/readme.md", "readme.md", "Project README and documentation", "markdown"],
        )
        .expect("Failed to insert into FTS");

    vault_db
        .conn
        .execute(
            "INSERT INTO files_fts (path, name, content, language) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["src/main.rs", "main.rs", "Rust main entry point", "rust"],
        )
        .expect("Failed to insert into FTS");

    // Test search by name - FTS searches content, name, and path
    // Should find both "notes/project.md" (name match) and "docs/readme.md" (content match "Project README")
    let mut stmt = vault_db
        .conn
        .prepare("SELECT path FROM files_fts WHERE files_fts MATCH ?1 LIMIT 100")
        .expect("Failed to prepare search");

    let results: Vec<String> = stmt
        .query_map(["project"], |row| row.get::<_, String>(0))
        .expect("Failed to query")
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to collect");

    assert_eq!(results.len(), 2, "Should find two files matching 'project' (name and content match)");
    assert!(results.contains(&"notes/project.md".to_string()));
    assert!(results.contains(&"docs/readme.md".to_string()));

    // Test search by extension
    let mut stmt = vault_db
        .conn
        .prepare("SELECT path FROM files_fts WHERE files_fts MATCH ?1 LIMIT 100")
        .expect("Failed to prepare search");

    let results: Vec<String> = stmt
        .query_map(["readme"], |row| row.get::<_, String>(0))
        .expect("Failed to query")
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to collect");

    assert_eq!(results.len(), 1, "Should find one file matching 'readme'");
    assert_eq!(results[0], "docs/readme.md");
}

#[test]
fn test_search_files_multiple_results() {
    // Create a temporary directory for the test vault
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let vault_path = temp_dir.path();

    // Initialize the vault database
    let vault_db = init_vault_db(vault_path).expect("Failed to initialize vault database");

    // Insert multiple files that contain "design"
    vault_db
        .conn
        .execute(
            "INSERT INTO files (path, name, extension, file_type, language, size_bytes, is_indexed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "docs/design-system.md",
                "design-system.md",
                "md",
                "markdown",
                "markdown",
                1024i64,
                1i32
            ],
        )
        .expect("Failed to insert design-system file");

    vault_db
        .conn
        .execute(
            "INSERT INTO files (path, name, extension, file_type, language, size_bytes, is_indexed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "notes/design-patterns.md",
                "design-patterns.md",
                "md",
                "markdown",
                "markdown",
                2048i64,
                1i32
            ],
        )
        .expect("Failed to insert design-patterns file");

    // Insert into FTS table
    vault_db
        .conn
        .execute(
            "INSERT INTO files_fts (path, name, content, language) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["docs/design-system.md", "design-system.md", "System design and components", "markdown"],
        )
        .expect("Failed to insert into FTS");

    vault_db
        .conn
        .execute(
            "INSERT INTO files_fts (path, name, content, language) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["notes/design-patterns.md", "design-patterns.md", "Design patterns and best practices", "markdown"],
        )
        .expect("Failed to insert into FTS");

    // Test search - should find both design files
    let mut stmt = vault_db
        .conn
        .prepare("SELECT path FROM files_fts WHERE files_fts MATCH ?1 LIMIT 100")
        .expect("Failed to prepare search");

    let results: Vec<String> = stmt
        .query_map(["design"], |row| row.get::<_, String>(0))
        .expect("Failed to query")
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to collect");

    assert_eq!(
        results.len(),
        2,
        "Should find two files matching 'design': {:?}",
        results
    );
    assert!(results.contains(&"docs/design-system.md".to_string()));
    assert!(results.contains(&"notes/design-patterns.md".to_string()));
}

#[test]
fn test_search_files_empty_query() {
    // Create a temporary directory for the test vault
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let vault_path = temp_dir.path();

    // Initialize the vault database
    let vault_db = init_vault_db(vault_path).expect("Failed to initialize vault database");

    // Insert a test file
    vault_db
        .conn
        .execute(
            "INSERT INTO files (path, name, extension, file_type, language, size_bytes, is_indexed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "notes/test.md",
                "test.md",
                "md",
                "markdown",
                "markdown",
                1024i64,
                1i32
            ],
        )
        .expect("Failed to insert test file");

    vault_db
        .conn
        .execute(
            "INSERT INTO files_fts (path, name, content, language) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["notes/test.md", "test.md", "Test content", "markdown"],
        )
        .expect("Failed to insert into FTS");

    // Test with empty query - should return empty results
    let mut stmt = vault_db
        .conn
        .prepare("SELECT path FROM files_fts WHERE files_fts MATCH ?1 LIMIT 100")
        .expect("Failed to prepare search");

    // Empty string queries typically match nothing in FTS5
    let results: Vec<String> = stmt
        .query_map([""], |row| row.get::<_, String>(0))
        .expect("Failed to query")
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default();

    assert_eq!(results.len(), 0, "Empty query should return no results");
}

#[test]
fn test_search_files_language_filter() {
    // Create a temporary directory for the test vault
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let vault_path = temp_dir.path();

    // Initialize the vault database
    let vault_db = init_vault_db(vault_path).expect("Failed to initialize vault database");

    // Insert Rust files
    vault_db
        .conn
        .execute(
            "INSERT INTO files (path, name, extension, file_type, language, size_bytes, is_indexed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "src/lib.rs",
                "lib.rs",
                "rs",
                "code",
                "rust",
                4096i64,
                1i32
            ],
        )
        .expect("Failed to insert rust lib");

    vault_db
        .conn
        .execute(
            "INSERT INTO files (path, name, extension, file_type, language, size_bytes, is_indexed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "src/main.py",
                "main.py",
                "py",
                "code",
                "python",
                2048i64,
                1i32
            ],
        )
        .expect("Failed to insert python file");

    // Insert into FTS table
    vault_db
        .conn
        .execute(
            "INSERT INTO files_fts (path, name, content, language) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["src/lib.rs", "lib.rs", "Rust library implementation", "rust"],
        )
        .expect("Failed to insert into FTS");

    vault_db
        .conn
        .execute(
            "INSERT INTO files_fts (path, name, content, language) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["src/main.py", "main.py", "Python main script", "python"],
        )
        .expect("Failed to insert into FTS");

    // Test search for rust language
    let mut stmt = vault_db
        .conn
        .prepare("SELECT path FROM files_fts WHERE files_fts MATCH ?1 LIMIT 100")
        .expect("Failed to prepare search");

    let results: Vec<String> = stmt
        .query_map(["rust"], |row| row.get::<_, String>(0))
        .expect("Failed to query")
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to collect");

    assert!(
        results.iter().any(|p| p.contains("lib.rs")),
        "Should find Rust file when searching for 'rust'"
    );
}
