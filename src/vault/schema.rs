/// SQLite schema for vault database
pub const SCHEMA: &str = r#"
-- Files table: stores file metadata
CREATE TABLE IF NOT EXISTS files (
    id INTEGER PRIMARY KEY,
    path TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    extension TEXT,
    file_type TEXT,
    language TEXT,
    size_bytes INTEGER,
    modified_at INTEGER,
    last_indexed_at INTEGER,
    is_indexed BOOLEAN DEFAULT 0
);

-- File links table: stores file relationships
CREATE TABLE IF NOT EXISTS file_links (
    id INTEGER PRIMARY KEY,
    source_id INTEGER NOT NULL,
    target_id INTEGER,
    link_type TEXT,
    FOREIGN KEY (source_id) REFERENCES files(id),
    FOREIGN KEY (target_id) REFERENCES files(id)
);

-- Word graph table: stores word co-occurrence data
CREATE TABLE IF NOT EXISTS word_graph (
    id INTEGER PRIMARY KEY,
    word1 TEXT NOT NULL,
    word2 TEXT NOT NULL,
    co_occurrence_count INTEGER DEFAULT 1,
    last_updated INTEGER
);

-- Index state table: stores indexing metadata
CREATE TABLE IF NOT EXISTS index_state (
    id INTEGER PRIMARY KEY,
    last_full_index INTEGER,
    last_incremental_index INTEGER,
    total_files_indexed INTEGER
);

-- Recent files table: stores recently opened files
CREATE TABLE IF NOT EXISTS recent_files (
    id INTEGER PRIMARY KEY,
    file_id INTEGER NOT NULL UNIQUE,
    opened_at INTEGER NOT NULL,
    FOREIGN KEY (file_id) REFERENCES files(id)
);

-- Full-text search virtual table
CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(path, name, content, language);

-- Indexes for optimized queries
CREATE INDEX IF NOT EXISTS idx_files_extension ON files(extension);
CREATE INDEX IF NOT EXISTS idx_files_language ON files(language);
CREATE INDEX IF NOT EXISTS idx_file_links_source ON file_links(source_id);
CREATE INDEX IF NOT EXISTS idx_file_links_target ON file_links(target_id);
CREATE INDEX IF NOT EXISTS idx_recent_files_opened ON recent_files(opened_at DESC);
"#;
