use std::collections::HashMap;

/// Inverted index mapping tokens to their source locations (doc_path, line_number).
/// Line numbers are 1-indexed.
#[derive(Debug, Default)]
pub struct InvertedIndex {
    index: HashMap<String, Vec<(String, u32)>>,
}

impl InvertedIndex {
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
        }
    }

    /// Index a single document's lines.
    /// Each (path, line) pair is stored at most once per token.
    pub fn add_document(&mut self, path: &str, lines: &[String]) {
        for (i, line) in lines.iter().enumerate() {
            let line_num = (i + 1) as u32; // 1-indexed
            let mut seen = std::collections::HashSet::new();
            for token in tokenize(line) {
                if seen.insert(token.clone()) {
                    self.index
                        .entry(token)
                        .or_default()
                        .push((path.to_string(), line_num));
                }
            }
        }
    }

    #[cfg(test)]
    pub fn lookup(&self, token: &str) -> Option<&Vec<(String, u32)>> {
        self.index.get(&token.to_lowercase())
    }

    pub fn token_count(&self) -> usize {
        self.index.len()
    }

    /// Find all (path, line_number) locations where a token contains the given
    /// substring. Returns deduplicated results sorted by (path, line).
    pub fn find_containing(&self, substring: &str) -> Vec<(String, u32)> {
        let mut seen = std::collections::BTreeSet::new();

        for (token, locations) in &self.index {
            if token.contains(substring) {
                for (path, line_num) in locations {
                    seen.insert((path.clone(), *line_num));
                }
            }
        }

        seen.into_iter().collect()
    }
}

/// Tokenize a line: lowercase, split on non-alphanumeric boundaries.
fn tokenize(line: &str) -> Vec<String> {
    line.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("Hello, World! This is a test.");
        assert_eq!(tokens, vec!["hello", "world", "this", "is", "a", "test"]);
    }

    #[test]
    fn test_tokenize_markdown() {
        let tokens = tokenize("## How to reset your password");
        assert!(tokens.contains(&"reset".to_string()));
        assert!(tokens.contains(&"password".to_string()));
    }

    #[test]
    fn test_index_and_lookup() {
        let mut idx = InvertedIndex::new();
        idx.add_document(
            "test.md",
            &[
                "Hello world".to_string(),
                "Goodbye world".to_string(),
            ],
        );

        let results = idx.lookup("world").unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], ("test.md".to_string(), 1));
        assert_eq!(results[1], ("test.md".to_string(), 2));
    }

    #[test]
    fn test_lookup_case_insensitive() {
        let mut idx = InvertedIndex::new();
        idx.add_document("test.md", &["Hello World".to_string()]);

        assert!(idx.lookup("hello").is_some());
        assert!(idx.lookup("HELLO").is_some());
    }

    #[test]
    fn test_lookup_miss() {
        let idx = InvertedIndex::new();
        assert!(idx.lookup("nonexistent").is_none());
    }
}
