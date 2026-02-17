import { readFileSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { initSync, MemexFS } from "../pkg/memexfs.js";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const wasmPath = resolve(__dirname, "../pkg/memexfs_bg.wasm");

let initialized = false;

function ensureInit() {
  if (!initialized) {
    const wasmBytes = readFileSync(wasmPath);
    initSync({ module: wasmBytes });
    initialized = true;
  }
}

/**
 * Load all .md files from a directory into a MemexFS instance.
 * @param {string} dir - Absolute or relative path to the directory.
 * @returns {MemexFS}
 */
export function loadFromDirectory(dir) {
  ensureInit();

  const absDir = resolve(dir);
  const files = readdirSync(absDir).filter((f) => f.endsWith(".md")).sort();
  const docs = files.map((f) => {
    const content = readFileSync(join(absDir, f), "utf-8");
    return [f, content];
  });

  return new MemexFS(JSON.stringify(docs));
}
