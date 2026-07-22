pub mod schema;

use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// Vault database connection wrapper
pub struct VaultDb {
    pub conn: Connection,
    #[allow(dead_code)]
    pub vault_root: PathBuf,
}

impl VaultDb {
    /// Create a new VaultDb instance
    pub fn new(conn: Connection, vault_root: PathBuf) -> Self {
        VaultDb { conn, vault_root }
    }
}

/// Initialize the vault database at vault_root/.tomarkdown/vault.db
pub fn init_vault_db(vault_root: &Path) -> Result<VaultDb, Box<dyn std::error::Error>> {
    // Create .tomarkdown directory if it doesn't exist
    let tomarkdown_dir = vault_root.join(".tomarkdown");
    std::fs::create_dir_all(&tomarkdown_dir)?;

    // Open or create SQLite database
    let db_path = tomarkdown_dir.join("vault.db");
    let conn = Connection::open(&db_path)?;

    // Execute schema
    conn.execute_batch(schema::SCHEMA)?;

    // Return VaultDb instance
    Ok(VaultDb::new(conn, vault_root.to_path_buf()))
}
