import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { loadFromDirectory } from "./loader.mjs";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const fixturesDir = resolve(__dirname, "../fixtures");

let fs;

describe("MemexFS via WASM", () => {
  it("loads all fixtures", () => {
    fs = loadFromDirectory(fixturesDir);
    assert.equal(fs.document_count(), 100);
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

  it("tool_definitions returns two tools", () => {
    const defs = JSON.parse(fs.tool_definitions());
    assert.ok(Array.isArray(defs));
    assert.equal(defs.length, 2);
    const names = defs.map((d) => d.name).sort();
    assert.deepEqual(names, ["grep", "read"]);
  });
});
