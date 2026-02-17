import { execSync } from "node:child_process";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { loadFromDirectory } from "./loader.mjs";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const fixturesDir = resolve(__dirname, "../fixtures");

const fs = loadFromDirectory(fixturesDir);
console.log(
  `Loaded ${fs.document_count()} documents, ${fs.token_count()} unique tokens\n`
);

const ITERATIONS = 1000;
const patterns = ["archive", "server", "download", "file", "command"];

console.log(`--- WASM grep (${ITERATIONS} iterations) ---`);
for (const pattern of patterns) {
  const start = performance.now();
  for (let i = 0; i < ITERATIONS; i++) {
    fs.grep(pattern);
  }
  const elapsed = performance.now() - start;
  const perOp = (elapsed / ITERATIONS).toFixed(4);
  console.log(`  "${pattern}": ${perOp} ms/op  (${elapsed.toFixed(1)} ms total)`);
}

console.log(`\n--- System grep -rn (${ITERATIONS} iterations) ---`);
for (const pattern of patterns) {
  const start = performance.now();
  for (let i = 0; i < ITERATIONS; i++) {
    try {
      execSync(`grep -rn "${pattern}" "${fixturesDir}"`, {
        stdio: "pipe",
        encoding: "utf-8",
      });
    } catch {
      // grep returns exit code 1 when no matches found
    }
  }
  const elapsed = performance.now() - start;
  const perOp = (elapsed / ITERATIONS).toFixed(4);
  console.log(`  "${pattern}": ${perOp} ms/op  (${elapsed.toFixed(1)} ms total)`);
}

console.log("\nDone.");
