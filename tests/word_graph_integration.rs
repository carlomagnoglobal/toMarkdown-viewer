//! Integration tests for the word graph feature
//! Tests the full workflow: indexing, querying, and graph data retrieval

use std::fs;
use std::path::PathBuf;

/// Get the path to the test fixture vault
fn fixture_vault() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("word_graph_test_vault")
}

#[test]
fn test_word_graph_end_to_end() {
    // Setup: Create test database in a temporary directory
    let temp_dir = std::env::temp_dir().join("word_graph_integration_test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    // Copy test markdown files from fixture to temp directory
    let fixture_path = fixture_vault();
    for entry in fs::read_dir(&fixture_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "md") {
            let file_name = path.file_name().unwrap();
            let dest = temp_dir.join(file_name);
            fs::copy(&path, &dest).unwrap();
        }
    }

    // Initialize the word graph database
    let db = to_markdown_gui::word_graph::db::WordGraphDb::new(&temp_dir)
        .expect("Failed to create word graph database");

    // Index the vault
    to_markdown_gui::word_graph::indexer::index_vault_full(&db, &temp_dir)
        .expect("Failed to index vault");

    // Verify word index contains expected words
    let top_words = to_markdown_gui::word_graph::queries::get_top_words(&db, 50)
        .expect("Failed to retrieve top words");

    assert!(!top_words.is_empty(), "Word graph should contain indexed words");

    // Check for specific words that should be in the fixture vault
    let word_list: Vec<&str> = top_words.iter().map(|(w, _)| w.as_str()).collect();

    // These words should be present in the fixture vault
    assert!(
        word_list.contains(&"test") || word_list.contains(&"word") || word_list.contains(&"graph"),
        "Expected at least one of [test, word, graph] in indexed words"
    );

    // Verify frequencies are reasonable (should be positive integers)
    for (word, frequency) in &top_words {
        assert!(
            *frequency > 0,
            "Word '{}' should have positive frequency, got {}",
            word,
            frequency
        );
    }

    // Get the IDs of indexed words
    let word_ids: Vec<i32> = top_words
        .iter()
        .take(10)
        .map(|(w, _)| {
            db.conn()
                .query_row("SELECT id FROM words WHERE word = ?1", [w], |row| row.get(0))
                .expect("Failed to get word ID")
        })
        .collect();

    assert!(!word_ids.is_empty(), "Should have retrieved word IDs");

    // Verify co-occurrence relationships
    let pairs = to_markdown_gui::word_graph::queries::get_word_pairs_for_graph(&db, &word_ids, 1)
        .expect("Failed to retrieve word pairs");

    // Co-occurrence pairs may or may not exist depending on the fixture content
    // This test just verifies the query doesn't panic
    for (_w1_id, _w2_id, count) in pairs {
        assert!(
            count > 0,
            "Co-occurrence count should be positive, got {}",
            count
        );
    }

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_word_graph_database_initialization() {
    // Setup temporary directory
    let temp_dir = std::env::temp_dir().join("word_graph_db_init_test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    // Create a simple test markdown file
    fs::write(
        temp_dir.join("test.md"),
        "# Test Note\n\nThis is a simple test document for word graph testing.",
    )
    .unwrap();

    // Initialize database
    let db = to_markdown_gui::word_graph::db::WordGraphDb::new(&temp_dir)
        .expect("Failed to create database");

    // Verify database schema was created
    let mut stmt = db
        .conn()
        .prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .unwrap();
    let table_names: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(Result::ok)
        .collect();

    assert!(table_names.contains(&"words".to_string()), "words table should exist");
    assert!(
        table_names.contains(&"co_occurrence".to_string()),
        "co_occurrence table should exist"
    );
    assert!(
        table_names.contains(&"index_state".to_string()),
        "index_state table should exist"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_word_graph_excludes_stopwords() {
    // Setup
    let temp_dir = std::env::temp_dir().join("word_graph_stopwords_test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    // Create a file with mostly stopwords
    fs::write(
        temp_dir.join("stopwords.md"),
        "the and is a to for that it with be was are or an from this at which have",
    )
    .unwrap();

    // Create a file with content words
    fs::write(
        temp_dir.join("content.md"),
        "markdown document testing integration index example",
    )
    .unwrap();

    // Index
    let db = to_markdown_gui::word_graph::db::WordGraphDb::new(&temp_dir).unwrap();
    to_markdown_gui::word_graph::indexer::index_vault_full(&db, &temp_dir).unwrap();

    // Verify stopwords are excluded
    let all_words = to_markdown_gui::word_graph::queries::get_top_words(&db, 100).unwrap();
    let word_list: Vec<&str> = all_words.iter().map(|(w, _)| w.as_str()).collect();

    // Common stopwords should not appear
    assert!(
        !word_list.contains(&"the"),
        "Stopword 'the' should not be indexed"
    );
    assert!(
        !word_list.contains(&"and"),
        "Stopword 'and' should not be indexed"
    );
    assert!(
        !word_list.contains(&"is"),
        "Stopword 'is' should not be indexed"
    );

    // Content words should appear
    assert!(word_list.contains(&"markdown"), "Content word 'markdown' should be indexed");
    assert!(
        word_list.contains(&"document"),
        "Content word 'document' should be indexed"
    );
    assert!(
        word_list.contains(&"testing"),
        "Content word 'testing' should be indexed"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}
