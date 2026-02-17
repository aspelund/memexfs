import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { initSync, MemexFS } from "../pkg/memexfs.js";
import { collectMdFiles } from "./collect.mjs";

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
 * Load all .md files recursively from a directory into a MemexFS instance.
 * @param {string} dir - Absolute or relative path to the directory.
 * @returns {MemexFS}
 */
export function loadFromDirectory(dir) {
  ensureInit();

  const absDir = resolve(dir);
  const docs = collectMdFiles(absDir, absDir).sort((a, b) =>
    a[0].localeCompare(b[0])
  );

  return new MemexFS(JSON.stringify(docs));
}
