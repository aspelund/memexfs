import { parentPort, workerData } from "node:worker_threads";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { initSync, MemexFS } from "../pkg/memexfs.js";
import { collectMdFiles } from "./collect.mjs";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const wasmPath = resolve(__dirname, "../pkg/memexfs_bg.wasm");

const wasmBytes = readFileSync(wasmPath);
initSync({ module: wasmBytes });

const absDir = resolve(workerData.dir);
const docs = collectMdFiles(absDir, absDir).sort((a, b) =>
  a[0].localeCompare(b[0])
);
const fs = new MemexFS(JSON.stringify(docs));

parentPort.postMessage({ type: "ready" });

parentPort.on("message", (msg) => {
  const { id, method, args } = msg;
  try {
    let result;
    switch (method) {
      case "grep":
        result = JSON.parse(fs.grep(...args));
        break;
      case "read":
        result = fs.read(...args);
        break;
      case "ls":
        result = JSON.parse(fs.ls(...args));
        break;
      case "toolDefinitions":
        result = JSON.parse(fs.tool_definitions());
        break;
      case "documentCount":
        result = fs.document_count();
        break;
      default:
        throw new Error(`unknown method: ${method}`);
    }
    parentPort.postMessage({ id, result });
  } catch (err) {
    parentPort.postMessage({ id, error: err.message });
  }
});
