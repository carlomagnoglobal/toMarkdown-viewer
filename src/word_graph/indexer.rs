use crate::word_graph::db::WordGraphDb;
use crate::word_graph::tokenizer::{tokenize, get_word_pairs};
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

pub fn index_vault_full(db: &WordGraphDb, vault_path: &Path) -> Result<()> {
    use log::info;

    // Find all markdown files
    let md_files = find_markdown_files(vault_path)?;
    info!("[WORD_GRAPH] Found {} markdown files to index", md_files.len());

    // Clear existing index
    {
        let tx = db.conn_mut().transaction()?;
        tx.execute("DELETE FROM co_occurrence", [])?;
        tx.execute("DELETE FROM words", [])?;
        tx.commit()?;
    }
    info!("[WORD_GRAPH] Cleared existing index");

    // Process files in batches
    let batch_size = 50; // Commit every 50 files
    for (batch_num, file_batch) in md_files.chunks(batch_size).enumerate() {
        info!("[WORD_GRAPH] Processing batch {} ({} files)", batch_num + 1, file_batch.len());

        let mut word_freq: HashMap<String, i32> = HashMap::new();
        let mut co_occur: HashMap<(String, String), (i32, Vec<String>)> = HashMap::new();

        // Index files in this batch
        for file_path in file_batch {
            let content = match std::fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(e) => {
                    info!("[WORD_GRAPH] Skipping {}: {}", file_path.display(), e);
                    continue;
                }
            };

            let words = tokenize(&content);
            let pairs = get_word_pairs(&words);

            for word in &words {
                *word_freq.entry(word.clone()).or_insert(0) += 1;
            }

            for (w1, w2) in pairs {
                let key = (w1, w2);
                let entry = co_occur.entry(key.clone()).or_insert((0, vec![]));
                entry.0 += 1;
                let rel_path = file_path.strip_prefix(vault_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| file_path.to_string_lossy().to_string());
                if !entry.1.contains(&rel_path) {
                    entry.1.push(rel_path);
                }
            }
        }

        // Commit this batch to database
        {
            let tx = db.conn_mut().transaction()?;

            for (word, freq) in word_freq {
                tx.execute(
                    "INSERT OR REPLACE INTO words (word, frequency) VALUES (?1, ?2)",
                    [word, freq.to_string()],
                )?;
            }

            for ((w1, w2), (count, notes)) in co_occur {
                let w1_id: i32 = tx.query_row(
                    "SELECT id FROM words WHERE word = ?1",
                    [&w1],
                    |row| row.get(0)
                )?;
                let w2_id: i32 = tx.query_row(
                    "SELECT id FROM words WHERE word = ?1",
                    [&w2],
                    |row| row.get(0)
                )?;
                let notes_json = serde_json::to_string(&notes)?;

                tx.execute(
                    "INSERT OR REPLACE INTO co_occurrence (word1_id, word2_id, count, notes_list_json) VALUES (?1, ?2, ?3, ?4)",
                    [w1_id.to_string(), w2_id.to_string(), count.to_string(), notes_json],
                )?;
            }

            tx.commit()?;
        }

        info!("[WORD_GRAPH] Batch {} committed", batch_num + 1);
    }

    // Update index state
    {
        let tx = db.conn_mut().transaction()?;
        tx.execute(
            "INSERT OR REPLACE INTO index_state (vault_path, last_full_index, changed_files_since) VALUES (?1, CURRENT_TIMESTAMP, ?2)",
            [vault_path.to_string_lossy().to_string(), "[]".to_string()],
        )?;
        tx.commit()?;
    }

    info!("[WORD_GRAPH] Full index completed successfully");
    Ok(())
}

pub fn index_vault_delta(db: &WordGraphDb, vault_path: &Path, changed_files: &[std::path::PathBuf]) -> Result<()> {
    if changed_files.is_empty() {
        return Ok(());
    }

    let tx = db.conn_mut().transaction()?;

    // Step 1: Remove old entries for changed files
    for file_path in changed_files {
        let rel_path = file_path.strip_prefix(vault_path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| file_path.to_string_lossy().to_string());

        // Find all co-occurrence pairs that reference this file and remove it from the list
        let mut stmt = tx.prepare(
            "SELECT word1_id, word2_id, notes_list_json FROM co_occurrence"
        )?;
        let pairs: Vec<(i32, i32, String)> = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?
            .collect::<Result<Vec<_>, _>>()?;

        for (w1_id, w2_id, notes_json) in pairs {
            let mut notes: Vec<String> = serde_json::from_str(&notes_json).unwrap_or_default();
            let orig_len = notes.len();
            notes.retain(|n| n != &rel_path);

            if notes.len() < orig_len {
                if notes.is_empty() {
                    tx.execute(
                        "DELETE FROM co_occurrence WHERE word1_id = ?1 AND word2_id = ?2",
                        [w1_id, w2_id],
                    )?;
                } else {
                    let updated_json = serde_json::to_string(&notes)?;
                    tx.execute(
                        "UPDATE co_occurrence SET notes_list_json = ?1 WHERE word1_id = ?2 AND word2_id = ?3",
                        [updated_json, w1_id.to_string(), w2_id.to_string()],
                    )?;
                }
            }
        }
    }

    // Step 2: Re-index changed files (add new entries)
    let mut word_freq_delta: HashMap<String, i32> = HashMap::new();
    let mut co_occur_delta: HashMap<(String, String), (i32, Vec<String>)> = HashMap::new();

    for file_path in changed_files {
        let content = std::fs::read_to_string(file_path)?;
        let words = tokenize(&content);
        let pairs = get_word_pairs(&words);

        for word in &words {
            *word_freq_delta.entry(word.clone()).or_insert(0) += 1;
        }

        for (w1, w2) in pairs {
            let key = (w1, w2);
            let entry = co_occur_delta.entry(key.clone()).or_insert((0, vec![]));
            entry.0 += 1;
            let rel_path = file_path.strip_prefix(vault_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| file_path.to_string_lossy().to_string());
            if !entry.1.contains(&rel_path) {
                entry.1.push(rel_path);
            }
        }
    }

    // Step 3: Update word frequencies and co-occurrence in database
    for (word, delta) in word_freq_delta {
        let existing: i32 = tx.query_row(
            "SELECT frequency FROM words WHERE word = ?1",
            [&word],
            |row| row.get(0)
        ).unwrap_or(0);

        if existing == 0 {
            tx.execute(
                "INSERT INTO words (word, frequency) VALUES (?1, ?2)",
                [word, delta.to_string()],
            )?;
        } else {
            tx.execute(
                "UPDATE words SET frequency = ?1 WHERE word = ?2",
                [(existing + delta).to_string(), word],
            )?;
        }
    }

    for ((w1, w2), (count_delta, notes)) in co_occur_delta {
        let w1_id: i32 = tx.query_row(
            "SELECT id FROM words WHERE word = ?1",
            [&w1],
            |row| row.get(0)
        )?;
        let w2_id: i32 = tx.query_row(
            "SELECT id FROM words WHERE word = ?1",
            [&w2],
            |row| row.get(0)
        )?;

        let existing: (i32, String) = tx.query_row(
            "SELECT count, notes_list_json FROM co_occurrence WHERE word1_id = ?1 AND word2_id = ?2",
            [w1_id, w2_id],
            |row| Ok((row.get(0)?, row.get(1)?))
        ).ok().unwrap_or((0, "[]".to_string()));

        let mut merged_notes: Vec<String> = serde_json::from_str(&existing.1).unwrap_or_default();
        for note in notes {
            if !merged_notes.contains(&note) {
                merged_notes.push(note);
            }
        }

        let notes_json = serde_json::to_string(&merged_notes)?;
        tx.execute(
            "INSERT OR REPLACE INTO co_occurrence (word1_id, word2_id, count, notes_list_json) VALUES (?1, ?2, ?3, ?4)",
            [w1_id.to_string(), w2_id.to_string(), (existing.0 + count_delta).to_string(), notes_json],
        )?;
    }

    tx.commit()?;
    Ok(())
}

fn find_markdown_files(vault_path: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut md_files = Vec::new();

    fn walk_dir(path: &Path, files: &mut Vec<std::path::PathBuf>) -> Result<()> {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() && !path.file_name().unwrap_or_default().to_string_lossy().starts_with('.') {
                walk_dir(&path, files)?;
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                files.push(path);
            }
        }
        Ok(())
    }

    walk_dir(vault_path, &mut md_files)?;
    Ok(md_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_temp_vault() -> Result<std::path::PathBuf> {
        let temp_dir = std::env::temp_dir().join("word_graph_indexer_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir)?;

        // Create test notes
        fs::write(temp_dir.join("note1.md"), "markdown editor vault")?;
        fs::write(temp_dir.join("note2.md"), "markdown document tool")?;

        Ok(temp_dir)
    }

    #[test]
    fn test_index_vault_full() -> Result<()> {
        let temp_dir = setup_temp_vault()?;
        let db = crate::word_graph::db::WordGraphDb::new(&temp_dir)?;

        index_vault_full(&db, &temp_dir)?;

        // Verify words stored
        let count: i32 = db.conn().query_row(
            "SELECT COUNT(*) FROM words",
            [],
            |row| row.get(0)
        )?;
        assert!(count > 0);

        // Verify co-occurrence stored
        let co_count: i32 = db.conn().query_row(
            "SELECT COUNT(*) FROM co_occurrence",
            [],
            |row| row.get(0)
        )?;
        assert!(co_count > 0);

        fs::remove_dir_all(&temp_dir).ok();
        Ok(())
    }
}
