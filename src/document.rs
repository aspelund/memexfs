use serde::{Deserialize, Serialize};

/// A single document stored as a path and its lines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub path: String,
    pub lines: Vec<String>,
    /// Pre-lowercased lines for fast case-insensitive search.
    pub lines_lower: Vec<String>,
}

impl Document {
    pub fn new(path: String, content: &str) -> Self {
        let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        let lines_lower: Vec<String> = lines.iter().map(|l| l.to_lowercase()).collect();
        Self { path, lines, lines_lower }
    }

    pub fn total_lines(&self) -> usize {
        self.lines.len()
    }

    /// Read lines with optional offset (1-indexed) and limit.
    /// Returns formatted text with line numbers.
    pub fn read(&self, offset: Option<usize>, limit: Option<usize>) -> String {
        let start = offset.unwrap_or(1).saturating_sub(1); // convert 1-indexed to 0-indexed
        if start >= self.lines.len() {
            return String::new();
        }

        let end = match limit {
            Some(lim) => (start + lim).min(self.lines.len()),
            None => self.lines.len(),
        };

        let width = end.to_string().len().max(3);
        self.lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let line_num = start + i + 1; // back to 1-indexed for display
                format!("{:>width$}  {}", line_num, line, width = width)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_new() {
        let doc = Document::new("test.md".into(), "line one\nline two\nline three");
        assert_eq!(doc.lines.len(), 3);
        assert_eq!(doc.lines[0], "line one");
    }

    #[test]
    fn test_read_full() {
        let doc = Document::new("test.md".into(), "# Title\n\nSome content");
        let result = doc.read(None, None);
        assert!(result.contains("# Title"));
        assert!(result.contains("Some content"));
    }

    #[test]
    fn test_read_with_offset_and_limit() {
        let doc = Document::new("test.md".into(), "line 1\nline 2\nline 3\nline 4\nline 5");
        let result = doc.read(Some(2), Some(2));
        assert!(result.contains("line 2"));
        assert!(result.contains("line 3"));
        assert!(!result.contains("line 1"));
        assert!(!result.contains("line 4"));
    }

    #[test]
    fn test_read_offset_beyond_end() {
        let doc = Document::new("test.md".into(), "only line");
        let result = doc.read(Some(100), None);
        assert!(result.is_empty());
    }
}
