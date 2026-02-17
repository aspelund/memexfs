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
    fn test_index_built_on_load() {
        let mut store = DocumentStore::new();
        store.load_documents(vec![("test.md".into(), "hello world".into())]);

        assert!(store.index().lookup("hello").is_some());
        assert!(store.index().lookup("world").is_some());
    }
}
