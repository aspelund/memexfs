import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { loadFromDirectory } from "./loader.mjs";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const fixturesDir = resolve(__dirname, "../fixtures");
const nestedDir = resolve(__dirname, "../fixtures/nested");

let fs;

describe("MemexFS via WASM", () => {
  it("loads all fixtures recursively", () => {
    fs = loadFromDirectory(fixturesDir);
    // 100 top-level .md files + 2 nested .md files
    assert.equal(fs.document_count(), 102);
  });

  it("has a positive token count", () => {
    assert.ok(fs.token_count() > 100, "should have many unique tokens");
  });

  it("grep finds 'archive' in tar.md", () => {
    const results = JSON.parse(fs.grep("archive"));
    assert.ok(results.length > 0, "should find matches");
    assert.ok(
      results.some((r) => r.path === "tar.md"),
      "tar.md should mention archive"
    );
  });

  it("grep with glob restricts results", () => {
    const results = JSON.parse(fs.grep("file", "tar.md"));
    assert.ok(results.length > 0, "should find matches");
    assert.ok(
      results.every((r) => r.path === "tar.md"),
      "glob should restrict to tar.md"
    );
  });

  it("reads a full document", () => {
    const content = fs.read("tar.md");
    assert.ok(content.includes("tar"));
    assert.ok(content.includes("Archiving"));
  });

  it("reads with offset and limit", () => {
    const content = fs.read("curl.md", 1, 3);
    assert.ok(content.includes("curl"));
    const lines = content.split("\n");
    assert.equal(lines.length, 3);
  });

  it("read returns error for missing file", () => {
    assert.throws(() => fs.read("nonexistent.md"), /document not found/);
  });

  it("grep supports regex patterns", () => {
    const results = JSON.parse(fs.grep("https?://"));
    assert.ok(results.length > 0, "should find URLs via regex");
  });

  it("call dispatches grep and read", () => {
    const grepResult = fs.call("grep", JSON.stringify({ pattern: "server" }));
    const parsed = JSON.parse(grepResult);
    assert.ok(Array.isArray(parsed));
    assert.ok(parsed.length > 0);

    const readResult = fs.call("read", JSON.stringify({ path: "git.md" }));
    assert.ok(readResult.includes("git"));
  });

  it("tool_definitions returns three tools", () => {
    const defs = JSON.parse(fs.tool_definitions());
    assert.ok(Array.isArray(defs));
    assert.equal(defs.length, 3);
    const names = defs.map((d) => d.name).sort();
    assert.deepEqual(names, ["grep", "ls", "read"]);
  });

  it("ls lists files and directories at root", () => {
    const entries = JSON.parse(fs.ls(""));
    assert.ok(entries.length > 0, "should list entries");
    assert.ok(
      entries.includes("nested/"),
      "should include nested/ subdirectory"
    );
    assert.ok(
      entries.some((e) => e.endsWith(".md")),
      "should include .md files"
    );
  });

  it("call dispatches ls", () => {
    const result = fs.call("ls", JSON.stringify({ path: "" }));
    const entries = JSON.parse(result);
    assert.ok(Array.isArray(entries));
    assert.ok(entries.length > 0);
  });
});

describe("loadFromDirectory recursive", () => {
  it("loads .md files recursively with relative paths", () => {
    const nested = loadFromDirectory(nestedDir);
    assert.equal(nested.document_count(), 2);

    // Should be able to read using relative paths
    const top = nested.read("top.md");
    assert.ok(top.includes("Top level doc"));

    const deep = nested.read("subdir/deep.md");
    assert.ok(deep.includes("nested inside subdir"));
  });

  it("ls shows subdirectories for recursively loaded docs", () => {
    const nested = loadFromDirectory(nestedDir);
    const entries = JSON.parse(nested.ls(""));
    assert.deepEqual(entries, ["subdir/", "top.md"]);
  });
});
