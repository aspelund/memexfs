use memexfs::MemexFsCore;
use std::fs;
use std::path::Path;

fn load_fixtures() -> MemexFsCore {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let mut docs: Vec<(String, String)> = Vec::new();

    for entry in fs::read_dir(&fixtures_dir).expect("fixtures/ directory must exist") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let content = fs::read_to_string(&path).unwrap();
            docs.push((name, content));
        }
    }

    docs.sort_by(|a, b| a.0.cmp(&b.0));

    let json = serde_json::to_string(&docs).unwrap();
    MemexFsCore::from_json(&json).unwrap()
}

#[test]
fn test_fixture_count() {
    let fs = load_fixtures();
    assert_eq!(fs.document_count(), 100);
}

#[test]
fn test_token_count() {
    let fs = load_fixtures();
    assert!(fs.token_count() > 100, "should have many unique tokens");
}

#[test]
fn test_grep_archive() {
    let fs = load_fixtures();
    let results = fs.grep("archive", None).unwrap();
    assert!(!results.is_empty(), "should find 'archive' in fixtures");
    assert!(
        results.iter().any(|r| r.path == "tar.md"),
        "tar.md should mention archive"
    );
}

#[test]
fn test_grep_download() {
    let fs = load_fixtures();
    let results = fs.grep("download", None).unwrap();
    assert!(!results.is_empty(), "should find 'download' in fixtures");
}

#[test]
fn test_grep_with_glob() {
    let fs = load_fixtures();
    let results = fs.grep("file", Some("tar.md")).unwrap();
    assert!(
        results.iter().all(|r| r.path == "tar.md"),
        "glob should restrict to tar.md only"
    );
}

#[test]
fn test_grep_regex() {
    let fs = load_fixtures();
    let results = fs.grep("https?://", None).unwrap();
    assert!(!results.is_empty(), "should find URLs via regex");
}

#[test]
fn test_read_tar() {
    let fs = load_fixtures();
    let content = fs.read("tar.md", None, None).unwrap();
    assert!(content.contains("tar"), "tar.md should contain 'tar'");
    assert!(
        content.contains("Archiving"),
        "tar.md should contain 'Archiving'"
    );
}

#[test]
fn test_read_with_offset_limit() {
    let fs = load_fixtures();
    let content = fs.read("curl.md", Some(1), Some(3)).unwrap();
    assert!(content.contains("curl"), "first lines of curl.md should contain 'curl'");
    // Should only have 3 lines
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 3, "limit=3 should return 3 lines");
}

#[test]
fn test_read_missing_file() {
    let fs = load_fixtures();
    let result = fs.read("nonexistent.md", None, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("document not found"));
}

#[test]
fn test_call_dispatch_grep() {
    let fs = load_fixtures();
    let result = fs
        .call("grep", r#"{"pattern": "server"}"#)
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.is_array());
    assert!(!parsed.as_array().unwrap().is_empty());
}

#[test]
fn test_call_dispatch_read() {
    let fs = load_fixtures();
    let result = fs
        .call("read", r#"{"path": "git.md"}"#)
        .unwrap();
    assert!(result.contains("git"));
}

#[test]
fn test_tool_definitions() {
    let fs = load_fixtures();
    let defs = fs.tool_definitions();
    let parsed: serde_json::Value = serde_json::from_str(&defs).unwrap();
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 3);
}
