use anyhow::{anyhow, Result};
use rusqlite::Connection;
use std::path::Path;
use std::cell::UnsafeCell;

pub struct WordGraphDb {
    conn: UnsafeCell<Connection>,
}

impl WordGraphDb {
    pub fn new(vault_path: &Path) -> Result<Self> {
        let db_path = vault_path.join(".tomarkdown_word_graph.db");
        let conn = Connection::open(&db_path)
            .map_err(|e| anyhow!("Failed to open word graph database: {}", e))?;

        // Enable WAL mode immediately after opening connection
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| anyhow!("Failed to enable WAL mode: {}", e))?;

        let db = Self { conn: UnsafeCell::new(conn) };
        db.initialize_schema()?;
        Ok(db)
    }

    pub fn initialize_schema(&self) -> Result<()> {
        unsafe {
            (*self.conn.get()).execute_batch(
                r#"
            CREATE TABLE IF NOT EXISTS words (
                id INTEGER PRIMARY KEY,
                word TEXT UNIQUE NOT NULL,
                frequency INTEGER NOT NULL,
                last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS co_occurrence (
                word1_id INTEGER NOT NULL,
                word2_id INTEGER NOT NULL,
                count INTEGER NOT NULL,
                notes_list_json TEXT,
                UNIQUE(word1_id, word2_id),
                FOREIGN KEY(word1_id) REFERENCES words(id),
                FOREIGN KEY(word2_id) REFERENCES words(id)
            );

            CREATE TABLE IF NOT EXISTS index_state (
                vault_path TEXT PRIMARY KEY,
                last_full_index TIMESTAMP,
                changed_files_since TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_words_frequency ON words(frequency);
            CREATE INDEX IF NOT EXISTS idx_index_state_vault ON index_state(vault_path);

            PRAGMA journal_mode=WAL;
            "#
            )
        }.map_err(|e| anyhow!("Failed to initialize schema: {}", e))?;

        Ok(())
    }

    pub fn conn(&self) -> &Connection {
        unsafe { &*self.conn.get() }
    }

    pub fn conn_mut(&self) -> &mut Connection {
        unsafe { &mut *self.conn.get() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_database_creation() {
        let temp_dir = std::env::temp_dir().join("word_graph_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let db = WordGraphDb::new(&temp_dir).unwrap();

        // Verify tables exist
        let mut stmt = db.conn()
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap();
        let table_names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        assert!(table_names.contains(&"words".to_string()));
        assert!(table_names.contains(&"co_occurrence".to_string()));
        assert!(table_names.contains(&"index_state".to_string()));

        fs::remove_dir_all(&temp_dir).ok();
    }
}
