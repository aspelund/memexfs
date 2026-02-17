mod document;
mod error;
mod index;
mod store;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use error::MemexError;
use store::DocumentStore;

/// A single grep match.
#[derive(Debug, Serialize, Deserialize)]
pub struct GrepResult {
    pub path: String,
    pub line: u32,
    pub content: String,
}

/// Core MemexFS logic, independent of WASM.
#[derive(Debug)]
pub struct MemexFsCore {
    store: DocumentStore,
}

impl MemexFsCore {
    pub fn from_json(docs_json: &str) -> Result<Self, MemexError> {
        let docs: Vec<(String, String)> = serde_json::from_str(docs_json)
            .map_err(|e| MemexError::new(&e.to_string()))?;

        if docs.is_empty() {
            return Err(MemexError::new("MemexError: no documents provided"));
        }

        let mut store = DocumentStore::new();
        store.load_documents(docs);

        Ok(Self { store })
    }

    pub fn grep(&self, pattern: &str, glob: Option<&str>) -> Result<Vec<GrepResult>, MemexError> {
        if pattern.is_empty() {
            return Err(MemexError::new("MemexError: empty search pattern"));
        }

        let max_results = 100;
        let mut results: Vec<GrepResult> = Vec::new();

        let is_simple = !has_regex_metacharacters(pattern);

        if is_simple {
            let tokens: Vec<&str> = pattern.split_whitespace().collect();
            let mut used_index = false;

            if let Some(first_token) = tokens.first() {
                if let Some(locations) = self.store.index().lookup(first_token) {
                    used_index = true;
                    for (doc_path, line_num) in locations {
                        if results.len() >= max_results {
                            break;
                        }

                        if let Some(g) = glob {
                            if !glob_match::glob_match(g, doc_path) {
                                continue;
                            }
                        }

                        if let Some(doc) = self.store.get_document(doc_path) {
                            let line_idx = (*line_num as usize).saturating_sub(1);
                            if line_idx < doc.lines.len() {
                                let line_content = &doc.lines[line_idx];
                                let line_lower = line_content.to_lowercase();
                                let pattern_lower = pattern.to_lowercase();
                                if tokens.len() == 1
                                    || line_lower.contains(&pattern_lower)
                                    || tokens
                                        .iter()
                                        .all(|t| line_lower.contains(&t.to_lowercase()))
                                {
                                    results.push(GrepResult {
                                        path: doc_path.clone(),
                                        line: *line_num,
                                        content: line_content.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }

            // Fallback: if the index had no exact token match, do a linear
            // substring scan so that partial-word patterns like "arch" still
            // match lines containing "archive".
            if !used_index || results.is_empty() {
                results.clear();
                let pattern_lower = pattern.to_lowercase();
                let paths = self.store.paths();

                for path in paths {
                    if results.len() >= max_results {
                        break;
                    }
                    if let Some(g) = glob {
                        if !glob_match::glob_match(g, path) {
                            continue;
                        }
                    }
                    if let Some(doc) = self.store.get_document(path) {
                        for (i, line) in doc.lines.iter().enumerate() {
                            if results.len() >= max_results {
                                break;
                            }
                            if line.to_lowercase().contains(&pattern_lower) {
                                results.push(GrepResult {
                                    path: path.to_string(),
                                    line: (i + 1) as u32,
                                    content: line.clone(),
                                });
                            }
                        }
                    }
                }
            }
        } else {
            let re = regex::RegexBuilder::new(pattern)
                .case_insensitive(true)
                .build()
                .map_err(|e| MemexError::new(&format!("MemexError: invalid regex: {}", e)))?;

            let paths = self.store.paths();

            for path in paths {
                if results.len() >= max_results {
                    break;
                }

                if let Some(g) = glob {
                    if !glob_match::glob_match(g, path) {
                        continue;
                    }
                }

                if let Some(doc) = self.store.get_document(path) {
                    for (i, line) in doc.lines.iter().enumerate() {
                        if results.len() >= max_results {
                            break;
                        }
                        if re.is_match(line) {
                            results.push(GrepResult {
                                path: path.to_string(),
                                line: (i + 1) as u32,
                                content: line.clone(),
                            });
                        }
                    }
                }
            }
        }

        results.sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
        Ok(results)
    }

    pub fn read(
        &self,
        path: &str,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> Result<String, MemexError> {
        let doc = self
            .store
            .get_document(path)
            .ok_or_else(|| MemexError::new(&format!("MemexError: document not found: {}", path)))?;

        Ok(doc.read(offset, limit))
    }

    pub fn call(&self, name: &str, params_json: &str) -> Result<String, MemexError> {
        match name {
            "grep" => {
                let params: GrepParams = serde_json::from_str(params_json)
                    .map_err(|e| MemexError::new(&e.to_string()))?;
                let results = self.grep(&params.pattern, params.glob.as_deref())?;
                serde_json::to_string(&results).map_err(|e| MemexError::new(&e.to_string()))
            }
            "read" => {
                let params: ReadParams = serde_json::from_str(params_json)
                    .map_err(|e| MemexError::new(&e.to_string()))?;
                self.read(
                    &params.path,
                    params.offset.map(|o| o as usize),
                    params.limit.map(|l| l as usize),
                )
            }
            _ => Err(MemexError::new(&format!(
                "MemexError: unknown tool: {}",
                name
            ))),
        }
    }

    pub fn tool_definitions(&self) -> String {
        serde_json::to_string(&tool_definitions_json()).unwrap()
    }

    pub fn document_count(&self) -> usize {
        self.store.document_count()
    }

    pub fn token_count(&self) -> usize {
        self.store.token_count()
    }
}

// ── WASM bindings ──────────────────────────────────────────────────

/// WASM-exported MemexFS. Thin wrapper over MemexFsCore that converts errors to JsError.
#[wasm_bindgen]
pub struct MemexFS {
    core: MemexFsCore,
}

#[wasm_bindgen]
impl MemexFS {
    #[wasm_bindgen(constructor)]
    pub fn new(docs_json: &str) -> Result<MemexFS, JsError> {
        let core = MemexFsCore::from_json(docs_json).map_err(|e| JsError::new(&e.message))?;
        Ok(MemexFS { core })
    }

    pub fn grep(&self, pattern: &str, glob: Option<String>) -> Result<String, JsError> {
        let results = self
            .core
            .grep(pattern, glob.as_deref())
            .map_err(|e| JsError::new(&e.message))?;
        serde_json::to_string(&results).map_err(|e| JsError::new(&e.to_string()))
    }

    pub fn read(
        &self,
        path: &str,
        offset: Option<u32>,
        limit: Option<u32>,
    ) -> Result<String, JsError> {
        self.core
            .read(path, offset.map(|o| o as usize), limit.map(|l| l as usize))
            .map_err(|e| JsError::new(&e.message))
    }

    pub fn tool_definitions(&self) -> String {
        self.core.tool_definitions()
    }

    pub fn call(&self, name: &str, params_json: &str) -> Result<String, JsError> {
        self.core
            .call(name, params_json)
            .map_err(|e| JsError::new(&e.message))
    }

    pub fn document_count(&self) -> usize {
        self.core.document_count()
    }

    pub fn token_count(&self) -> usize {
        self.core.token_count()
    }
}

// ── Helpers ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GrepParams {
    pattern: String,
    glob: Option<String>,
}

#[derive(Deserialize)]
struct ReadParams {
    path: String,
    offset: Option<u32>,
    limit: Option<u32>,
}

fn has_regex_metacharacters(pattern: &str) -> bool {
    pattern.contains(|c: char| {
        matches!(
            c,
            '|' | '*'
                | '+'
                | '?'
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | '\\'
                | '^'
                | '$'
                | '.'
        )
    })
}

fn tool_definitions_json() -> serde_json::Value {
    serde_json::json!([
        {
            "name": "grep",
            "description": "Search for a pattern across all documents. Returns matching file paths, line numbers, and content. Use this to find relevant documents before reading them.",
            "parameters": {
                "pattern": { "type": "string", "description": "Search pattern (supports regex)" },
                "glob": { "type": "string", "description": "Optional file pattern filter, e.g. 'billing/**/*.md'" }
            },
            "required": ["pattern"]
        },
        {
            "name": "read",
            "description": "Read the contents of a document. Returns the full document or a specific line range. Use this after grep to get the full context of a matching document.",
            "parameters": {
                "path": { "type": "string", "description": "Document path relative to the knowledge base root" },
                "offset": { "type": "number", "description": "Line number to start reading from (1-indexed)" },
                "limit": { "type": "number", "description": "Number of lines to return" }
            },
            "required": ["path"]
        }
    ])
}

// ── Tests ──────────────────────────────────────────────────────────
// Tests use MemexFsCore directly to avoid JsError on non-wasm targets.

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fs() -> MemexFsCore {
        let docs = serde_json::to_string(&vec![
            ("account/password-reset.md", "# Password Reset\n\n## How to reset your password\n\n1. Go to Settings\n2. Click Reset Password"),
            ("billing/refund.md", "# Refunds\n\nTo request a refund, contact support.\n\nRefunds are processed within 5 business days."),
        ]).unwrap();
        MemexFsCore::from_json(&docs).unwrap()
    }

    #[test]
    fn test_grep_simple() {
        let fs = make_fs();
        let results = fs.grep("password", None).unwrap();
        assert!(!results.is_empty());
        assert!(results
            .iter()
            .any(|r| r.path == "account/password-reset.md"));
    }

    #[test]
    fn test_grep_with_glob() {
        let fs = make_fs();
        let results = fs.grep("refund", Some("billing/**/*.md")).unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.path.starts_with("billing/")));
    }

    #[test]
    fn test_grep_regex() {
        let fs = make_fs();
        let results = fs.grep("reset|refund", None).unwrap();
        assert!(results.len() >= 2);
    }

    #[test]
    fn test_grep_case_insensitive() {
        let fs = make_fs();
        let results = fs.grep("PASSWORD", None).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_read_full() {
        let fs = make_fs();
        let content = fs.read("billing/refund.md", None, None).unwrap();
        assert!(content.contains("# Refunds"));
        assert!(content.contains("5 business days"));
    }

    #[test]
    fn test_read_with_offset_limit() {
        let fs = make_fs();
        let content = fs.read("billing/refund.md", Some(3), Some(1)).unwrap();
        assert!(content.contains("request a refund"));
        assert!(!content.contains("# Refunds"));
    }

    #[test]
    fn test_read_not_found() {
        let fs = make_fs();
        let result = fs.read("nonexistent.md", None, None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("document not found"));
    }

    #[test]
    fn test_call_dispatch() {
        let fs = make_fs();
        let result = fs.call("grep", r#"{"pattern": "refund"}"#).unwrap();
        assert!(result.contains("refund"));

        let result = fs
            .call("read", r#"{"path": "billing/refund.md"}"#)
            .unwrap();
        assert!(result.contains("# Refunds"));
    }

    #[test]
    fn test_call_unknown_tool() {
        let fs = make_fs();
        let result = fs.call("delete", r#"{}"#);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("unknown tool"));
    }

    #[test]
    fn test_tool_definitions() {
        let fs = make_fs();
        let defs = fs.tool_definitions();
        let parsed: serde_json::Value = serde_json::from_str(&defs).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_empty_docs() {
        let result = MemexFsCore::from_json("[]");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("no documents"));
    }

    #[test]
    fn test_invalid_json() {
        let result = MemexFsCore::from_json("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_document_count() {
        let fs = make_fs();
        assert_eq!(fs.document_count(), 2);
    }

    #[test]
    fn test_grep_max_results() {
        // Build a filesystem with many matching lines
        let mut docs = Vec::new();
        for i in 0..200 {
            docs.push((
                format!("doc_{}.md", i),
                "keyword match here\nkeyword match again".to_string(),
            ));
        }
        let json = serde_json::to_string(&docs).unwrap();
        let fs = MemexFsCore::from_json(&json).unwrap();
        let results = fs.grep("keyword", None).unwrap();
        assert_eq!(results.len(), 100); // capped at max
    }

    // Bug reproduction: substring matching
    #[test]
    fn test_grep_substring_in_token() {
        // "arch" is a substring of "archive" but not a standalone token
        let docs = serde_json::to_string(&vec![
            ("test.md", "This is an archive of data"),
        ]).unwrap();
        let fs = MemexFsCore::from_json(&docs).unwrap();
        let results = fs.grep("arch", None).unwrap();
        assert!(!results.is_empty(), "should find 'arch' inside 'archive'");
    }

    // Bug reproduction: duplicate matches per line
    #[test]
    fn test_grep_no_duplicate_lines() {
        // "file" appears twice on the same line
        let docs = serde_json::to_string(&vec![
            ("test.md", "copy file to file destination"),
        ]).unwrap();
        let fs = MemexFsCore::from_json(&docs).unwrap();
        let results = fs.grep("file", None).unwrap();
        assert_eq!(results.len(), 1, "should return one result per line, not per occurrence");
    }
}
