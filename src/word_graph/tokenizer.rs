pub const STOPWORDS: &[&str] = &[
    "the", "and", "is", "a", "in", "to", "of", "for", "that", "it", "as", "on", "by",
    "with", "be", "was", "are", "or", "an", "from", "this", "at", "which", "have",
    "has", "had", "do", "does", "did", "not", "no", "can", "could", "will", "would",
    "should", "may", "might", "must", "shall", "been", "being", "am", "i", "you",
    "he", "she", "we", "they", "him", "her", "us", "them", "my", "your", "his",
    "her", "its", "our", "their", "what", "when", "where", "why", "how", "all",
    "each", "every", "both", "few", "more", "some", "such", "no", "nor", "only",
    "own", "same", "so", "than", "too", "very", "just", "can", "now", "but",
];

const MIN_WORD_LENGTH: usize = 3;

pub fn tokenize(text: &str) -> Vec<String> {
    let mut tokens: Vec<String> = text
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| {
            word.len() >= MIN_WORD_LENGTH && !STOPWORDS.contains(&word)
        })
        .map(|word| word.to_string())
        .collect();
    tokens.sort();
    tokens.dedup();
    tokens
}

pub fn get_word_pairs(words: &[String]) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for (i, word1) in words.iter().enumerate() {
        for word2 in words.iter().skip(i + 1) {
            if word1 < word2 {
                pairs.push((word1.clone(), word2.clone()));
            } else if word2 < word1 {
                pairs.push((word2.clone(), word1.clone()));
            }
        }
    }
    pairs.sort();
    pairs.dedup();
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_extracts_words() {
        let text = "Hello world, this is a test!";
        let tokens = tokenize(text);
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
    }

    #[test]
    fn test_tokenize_filters_stopwords() {
        let text = "the and is a to for";
        let tokens = tokenize(text);
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_tokenize_enforces_min_length() {
        let text = "ab abc abcd";
        let tokens = tokenize(text);
        assert!(!tokens.contains(&"ab".to_string()));
        assert!(tokens.contains(&"abc".to_string()));
        assert!(tokens.contains(&"abcd".to_string()));
    }

    #[test]
    fn test_tokenize_lowercases() {
        let text = "HELLO Hello hello";
        let tokens = tokenize(text);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], "hello");
    }

    #[test]
    fn test_get_word_pairs() {
        let words = vec!["markdown".to_string(), "editor".to_string(), "note".to_string()];
        let pairs = get_word_pairs(&words);
        assert_eq!(pairs.len(), 3);
        assert!(pairs.contains(&("editor".to_string(), "markdown".to_string())));
        assert!(pairs.contains(&("editor".to_string(), "note".to_string())));
        assert!(pairs.contains(&("markdown".to_string(), "note".to_string())));
    }

    #[test]
    fn test_get_word_pairs_deduplicates() {
        let words = vec!["word".to_string(), "word".to_string(), "other".to_string()];
        let pairs = get_word_pairs(&words);
        let matching_pairs: Vec<_> = pairs.iter()
            .filter(|(w1, w2)| (w1 == "other" && w2 == "word") || (w1 == "word" && w2 == "other"))
            .collect();
        assert_eq!(matching_pairs.len(), 1);
    }
}
