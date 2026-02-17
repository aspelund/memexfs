use std::collections::HashMap;

use crate::document::Document;
use crate::index::InvertedIndex;

/// The in-memory document store + inverted index.
#[derive(Debug)]
pub struct DocumentStore {
    docs: HashMap<String, Document>,
    index: InvertedIndex,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self {
            docs: HashMap::new(),
            index: InvertedIndex::new(),
        }
    }

    /// Load documents from a serialized list of (path, content) pairs.
    pub fn load_documents(&mut self, documents: Vec<(String, String)>) {
        for (path, content) in documents {
            let doc = Document::new(path.clone(), &content);
            self.index.add_document(&path, &doc.lines);
            self.docs.insert(path, doc);
        }
    }

    pub fn get_document(&self, path: &str) -> Option<&Document> {
        self.docs.get(path)
    }

    pub fn document_count(&self) -> usize {
        self.docs.len()
    }

    pub fn token_count(&self) -> usize {
        self.index.token_count()
    }

    pub fn index(&self) -> &InvertedIndex {
        &self.index
    }

    /// Return all document paths, sorted.
    pub fn paths(&self) -> Vec<&str> {
        let mut paths: Vec<&str> = self.docs.keys().map(|s| s.as_str()).collect();
        paths.sort();
        paths
    }

    /// List immediate children of a virtual directory path.
    /// Returns file names and subdirectory names (with trailing `/`), sorted.
    pub fn ls(&self, dir: &str) -> Vec<String> {
        // Normalize: ensure prefix ends with '/' (or is empty for root)
        let prefix = if dir.is_empty() || dir == "/" || dir == "." {
            String::new()
        } else if dir.ends_with('/') {
            dir.to_string()
        } else {
            format!("{}/", dir)
        };

        let mut entries = std::collections::BTreeSet::new();

        for path in self.docs.keys() {
            let Some(rest) = path.strip_prefix(&prefix) else {
                // For root listing (empty prefix), rest == full path
                if !prefix.is_empty() {
                    continue;
                }
                // This shouldn't happen since strip_prefix("") always succeeds
                continue;
            };

            // rest is what comes after the prefix
            if let Some(slash_pos) = rest.find('/') {
                // There's a subdirectory
                let dir_name = format!("{}/", &rest[..slash_pos]);
                entries.insert(dir_name);
            } else {
                // Direct child file
                entries.insert(rest.to_string());
            }
        }

        entries.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_and_get() {
        let mut store = DocumentStore::new();
        store.load_documents(vec![
            ("a.md".into(), "Hello world".into()),
            ("b.md".into(), "Goodbye world".into()),
        ]);

        assert_eq!(store.document_count(), 2);
        assert!(store.get_document("a.md").is_some());
        assert!(store.get_document("missing.md").is_none());
    }

    #[test]
    fn test_ls_root() {
        let mut store = DocumentStore::new();
        store.load_documents(vec![
            ("dir/a.md".into(), "hello".into()),
            ("dir/b.md".into(), "world".into()),
            ("top.md".into(), "top".into()),
        ]);
        let entries = store.ls("");
        assert_eq!(entries, vec!["dir/", "top.md"]);
    }

    #[test]
    fn test_ls_subdir() {
        let mut store = DocumentStore::new();
        store.load_documents(vec![
            ("dir/a.md".into(), "hello".into()),
            ("dir/sub/b.md".into(), "world".into()),
        ]);
        let entries = store.ls("dir");
        assert_eq!(entries, vec!["a.md", "sub/"]);
    }

    #[test]
    fn test_index_built_on_load() {
        let mut store = DocumentStore::new();
        store.load_documents(vec![("test.md".into(), "hello world".into())]);

        assert!(store.index().lookup("hello").is_some());
        assert!(store.index().lookup("world").is_some());
    }
}
