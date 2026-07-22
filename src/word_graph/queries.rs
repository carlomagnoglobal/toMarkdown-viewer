use crate::word_graph::db::WordGraphDb;
use anyhow::Result;
use rusqlite::params;

pub fn get_top_words(db: &WordGraphDb, limit: usize) -> Result<Vec<(String, i32)>> {
    let mut stmt = db.conn().prepare(
        "SELECT word, frequency FROM words ORDER BY frequency DESC LIMIT ?"
    )?;

    let words = stmt.query_map(params![limit as i32], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(words)
}

pub fn get_word_pairs_for_graph(db: &WordGraphDb, word_ids: &[i32], threshold: i32) -> Result<Vec<(i32, i32, i32)>> {
    if word_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = word_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!(
        "SELECT word1_id, word2_id, count FROM co_occurrence WHERE count >= ? AND (word1_id IN ({}) OR word2_id IN ({}))",
        placeholders, placeholders
    );

    let mut stmt = db.conn().prepare(&query)?;

    // Build parameters: [threshold, ...word_ids, ...word_ids]
    let mut param_vec: Vec<&dyn rusqlite::ToSql> = vec![&threshold];
    let word_id_refs: Vec<&dyn rusqlite::ToSql> = word_ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
    param_vec.extend(&word_id_refs);
    param_vec.extend(&word_id_refs);

    let pairs = stmt.query_map(param_vec.as_slice(), |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(pairs)
}

pub fn get_notes_for_word(db: &WordGraphDb, word: &str) -> Result<Vec<String>> {
    let notes_json: String = db.conn().query_row(
        "SELECT COALESCE(GROUP_CONCAT(DISTINCT json_each.value), '[]') FROM co_occurrence, json_each(co_occurrence.notes_list_json) WHERE word1_id IN (SELECT id FROM words WHERE word = ?) OR word2_id IN (SELECT id FROM words WHERE word = ?)",
        params![word, word],
        |row| row.get(0)
    ).unwrap_or_else(|_| "[]".to_string());

    let notes: Vec<String> = serde_json::from_str(&notes_json).unwrap_or_default();
    Ok(notes)
}

#[allow(dead_code)]
pub fn get_co_occurrence_partners(db: &WordGraphDb, word: &str, limit: usize) -> Result<Vec<(String, i32)>> {
    let mut stmt = db.conn().prepare(
        "SELECT w.word, c.count FROM co_occurrence c
         JOIN words w1 ON c.word1_id = w1.id
         JOIN words w2 ON c.word2_id = w2.id
         JOIN words w ON (w.id = c.word2_id AND w1.word = ?) OR (w.id = c.word1_id AND w2.word = ?)
         ORDER BY c.count DESC LIMIT ?"
    )?;

    let partners = stmt.query_map(params![word, word, limit as i32], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(partners)
}

pub fn get_last_updated_timestamp(db: &WordGraphDb) -> Result<Option<String>> {
    let timestamp: Option<String> = db.conn().query_row(
        "SELECT last_full_index FROM index_state ORDER BY last_full_index DESC LIMIT 1",
        [],
        |row| row.get(0)
    ).ok();

    Ok(timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_get_top_words() -> Result<()> {
        let temp_dir = std::env::temp_dir().join("word_graph_queries_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir)?;
        fs::write(temp_dir.join("note.md"), "markdown editor vault markdown")?;

        let db = crate::word_graph::db::WordGraphDb::new(&temp_dir)?;
        crate::word_graph::indexer::index_vault_full(&db, &temp_dir)?;

        let top_words = get_top_words(&db, 5)?;
        assert!(!top_words.is_empty());
        assert!(top_words[0].0 == "markdown" || top_words[0].0 == "editor" || top_words[0].0 == "vault");

        fs::remove_dir_all(&temp_dir).ok();
        Ok(())
    }
}
