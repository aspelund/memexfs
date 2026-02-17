import { describe, it, after } from "node:test";
import assert from "node:assert/strict";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createPool } from "./pool.mjs";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const fixturesDir = resolve(__dirname, "../fixtures");
const nestedDir = resolve(__dirname, "../fixtures/nested");

describe("pool", () => {
  let pool;

  after(async () => {
    if (pool) await pool.terminate();
  });

  it("creates a pool and counts documents", async () => {
    pool = await createPool(fixturesDir, { workers: 2 });
    const count = await pool.documentCount();
    assert.equal(count, 102);
  });

  it("grep returns results", async () => {
    const results = await pool.grep("archive");
    assert.ok(Array.isArray(results));
    assert.ok(results.length > 0);
    assert.ok(results.some((r) => r.path === "tar.md"));
  });

  it("grep with glob restricts results", async () => {
    const results = await pool.grep("file", "tar.md");
    assert.ok(results.length > 0);
    assert.ok(results.every((r) => r.path === "tar.md"));
  });

  it("read returns document content", async () => {
    const content = await pool.read("tar.md");
    assert.ok(content.includes("tar"));
    assert.ok(content.includes("Archiving"));
  });

  it("read with offset and limit", async () => {
    const content = await pool.read("curl.md", 1, 3);
    assert.ok(content.includes("curl"));
    assert.equal(content.split("\n").length, 3);
  });

  it("read rejects for missing file", async () => {
    await assert.rejects(() => pool.read("nonexistent.md"), /document not found/);
  });

  it("ls lists entries", async () => {
    const entries = await pool.ls("");
    assert.ok(Array.isArray(entries));
    assert.ok(entries.includes("nested/"));
    assert.ok(entries.some((e) => e.endsWith(".md")));
  });

  it("toolDefinitions returns three tools", async () => {
    const defs = await pool.toolDefinitions();
    assert.ok(Array.isArray(defs));
    assert.equal(defs.length, 3);
    const names = defs.map((d) => d.name).sort();
    assert.deepEqual(names, ["grep", "ls", "read"]);
  });

  it("dispatches concurrent requests across workers", async () => {
    const promises = [];
    for (let i = 0; i < 10; i++) {
      promises.push(pool.grep("file"));
    }
    const results = await Promise.all(promises);
    for (const r of results) {
      assert.ok(Array.isArray(r));
      assert.ok(r.length > 0);
    }
  });
});

describe("pool with nested directory", () => {
  let pool;

  after(async () => {
    if (pool) await pool.terminate();
  });

  it("loads nested fixtures", async () => {
    pool = await createPool(nestedDir, { workers: 1 });
    const count = await pool.documentCount();
    assert.equal(count, 2);

    const content = await pool.read("top.md");
    assert.ok(content.includes("Top level doc"));

    const deep = await pool.read("subdir/deep.md");
    assert.ok(deep.includes("nested inside subdir"));
  });
});

describe("pool terminate", () => {
  it("rejects calls after terminate", async () => {
    const pool = await createPool(nestedDir, { workers: 1 });
    await pool.terminate();
    await assert.rejects(() => pool.grep("test"), /pool is terminated/);
  });
});
