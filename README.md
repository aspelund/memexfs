# memexfs

A WASM virtual filesystem exposing `grep` and `read` over markdown — two tools, sub-millisecond, zero dependencies.

## Why

RAG is overkill for < 500 documents. You don't need embeddings, vector databases, chunking strategies, or similarity thresholds to answer "which doc answers this question?" — you need grep.

`memexfs` loads a folder of markdown into memory, builds a fast index, and exposes exactly two operations: `grep` and `read`. Give these as tools to any LLM capable of tool calling (Claude Haiku, GPT-4o-mini, etc.) and you have a customer service agent, documentation assistant, or knowledge base — without the RAG pipeline.

The LLM **is** your re-ranker. It reads grep results, picks what to read deeper, and answers. That's the whole system.

## Install

```bash
npm install memexfs
```

## Quick start

### Browser

```js
import init, { MemexFS } from "memexfs";

// Initialize the WASM module (fetches the .wasm file automatically)
await init();

// Build the docs array: [[path, content], ...]
const docs = [
  ["account/password-reset.md", "# Password Reset\n\nGo to Settings > Reset Password."],
  ["billing/refund.md", "# Refunds\n\nContact support to request a refund."],
];

const fs = new MemexFS(JSON.stringify(docs));

// Search across all documents
const results = JSON.parse(fs.grep("password"));
// [{ path: "account/password-reset.md", line: 1, content: "# Password Reset" }]

// Read a specific document (returns line-numbered text)
const content = fs.read("account/password-reset.md");

// Read a specific section (offset is 1-indexed)
const section = fs.read("account/password-reset.md", 2, 5);
```

### Node.js

```js
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { initSync, MemexFS } from "memexfs";

// Initialize WASM synchronously in Node
const wasmPath = new URL("memexfs_bg.wasm", import.meta.resolve("memexfs"));
initSync({ module: readFileSync(wasmPath) });

// Load documents from a directory
const dir = "./docs";
const docs = readdirSync(dir)
  .filter(f => f.endsWith(".md"))
  .map(f => [f, readFileSync(join(dir, f), "utf-8")]);

const fs = new MemexFS(JSON.stringify(docs));

const results = JSON.parse(fs.grep("password reset"));
```

## API

### `new MemexFS(docs_json: string)`

Creates a new instance. `docs_json` is a JSON-serialized array of `[path, content]` tuples:

```js
const fs = new MemexFS(JSON.stringify([
  ["path/to/doc.md", "# Title\n\nContent here."],
  // ...
]));
```

### `fs.grep(pattern: string, glob?: string): string`

Searches all documents for `pattern`. Returns a JSON string of matches:

```js
const results = JSON.parse(fs.grep("password"));
// [{ path: "account/reset.md", line: 3, content: "## How to reset your password" }]

// With glob filter — only search billing docs
const filtered = JSON.parse(fs.grep("refund", "billing/*.md"));
```

- **Case-insensitive** — all searches are case-insensitive, both simple and regex
- Simple patterns use the inverted index (fast path); falls back to substring scan for partial-word matches
- Regex patterns (`|`, `*`, `+`, `?`, `.`, etc.) do a linear scan across all documents
- Max 100 results, sorted by path then line number
- Returns one result per matching line (not per occurrence)

### `fs.read(path: string, offset?: number, limit?: number): string`

Reads a document. Returns line-numbered text.

```js
const full = fs.read("billing/refund.md");
//   1  # Refunds
//   2
//   3  Contact support to request a refund.

const slice = fs.read("billing/refund.md", 3, 1);
//   3  Contact support to request a refund.
```

- `offset` is 1-indexed
- Throws if the path doesn't exist

### `fs.call(name: string, params_json: string): string`

Tool dispatcher for LLM integration. Accepts `"grep"` or `"read"` as the tool name:

```js
const result = fs.call("grep", JSON.stringify({ pattern: "reset", glob: "account/*.md" }));
const content = fs.call("read", JSON.stringify({ path: "account/reset.md", offset: 1, limit: 10 }));
```

### `fs.tool_definitions(): string`

Returns a JSON string with the tool definitions, ready to pass to an LLM:

```js
const tools = JSON.parse(fs.tool_definitions());
// [{ name: "grep", description: "...", parameters: {...} }, { name: "read", ... }]
```

### `fs.document_count(): number`

Returns the number of loaded documents.

### `fs.token_count(): number`

Returns the number of unique tokens in the inverted index.

## LLM tool definitions

Hand these to your LLM and let it work:

```json
[
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
]
```

## Example: customer service agent with Claude

```js
import Anthropic from "@anthropic-ai/sdk";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { initSync, MemexFS } from "memexfs";

// Initialize WASM
const wasmPath = new URL("memexfs_bg.wasm", import.meta.resolve("memexfs"));
initSync({ module: readFileSync(wasmPath) });

// Load knowledge base
const dir = "./knowledge-base";
const docs = readdirSync(dir)
  .filter(f => f.endsWith(".md"))
  .map(f => [f, readFileSync(join(dir, f), "utf-8")]);
const fs = new MemexFS(JSON.stringify(docs));

// Set up Claude with memexfs tools
const client = new Anthropic();
const tools = JSON.parse(fs.tool_definitions());

const response = await client.messages.create({
  model: "claude-haiku-4-5-20251001",
  max_tokens: 1024,
  system:
    "You are a customer service agent. Use grep to find relevant docs, then read to get full context. Answer based on the documentation only.",
  tools,
  messages: [{ role: "user", content: "How do I cancel my subscription?" }],
});

// Handle tool calls
for (const block of response.content) {
  if (block.type === "tool_use") {
    const result = fs.call(block.name, JSON.stringify(block.input));
    console.log(result);
  }
}
```

## Performance

Benchmarked against 100 real markdown files (tldr-pages command docs):

| Operation | WASM | System `grep -rn` | Speedup |
|-----------|------|--------------------|---------|
| `grep("archive")` | 0.015 ms | 3.96 ms | ~260x |
| `grep("server")` | 0.010 ms | 3.90 ms | ~400x |
| `grep("file")` | 0.028 ms | 3.78 ms | ~135x |

Targets for < 500 markdown documents:

| Operation | Target |
|-----------|--------|
| `MemexFS()` init | < 100ms |
| `grep(pattern)` | < 1ms |
| `read(path)` | < 1ms |
| Memory footprint | < 50 MB |

Run benchmarks yourself:

```bash
make bench
```

## Development

```bash
# Prerequisites: Rust, wasm-pack, Node.js >= 18

# Run Rust unit + integration tests (37 tests)
make test

# Build WASM + run Node.js tests (10 tests)
make test-node

# Run everything
make test-all

# Benchmarks (WASM grep vs system grep, 1000 iterations)
make bench

# Clean build artifacts
make clean
```

### Project structure

```
memexfs/
├── src/              # Rust source
│   ├── lib.rs        # MemexFsCore + MemexFS (WASM bindings)
│   ├── document.rs   # Document storage + line-numbered read
│   ├── index.rs      # Inverted index for fast token lookup
│   ├── store.rs      # DocumentStore combining docs + index
│   └── error.rs      # MemexError type
├── tests/
│   └── fixtures.rs   # Integration tests against real .md files
├── fixtures/         # 100 .md files from tldr-pages (test data)
├── js/
│   ├── loader.mjs    # Node.js helper: loadFromDirectory()
│   ├── loader.test.mjs  # Node.js tests (node:test, zero deps)
│   └── bench.mjs     # Performance benchmarks
├── pkg/              # wasm-pack output (gitignored)
├── docs/             # Architecture & spec docs
├── Cargo.toml
├── Makefile
└── package.json
```

## Design constraints

- **Two operations only.** grep and read. Nothing else.
- **Read-only.** No writes, no mutations, no state changes after init.
- **In-memory.** Everything loaded at init. No disk I/O after startup.
- **Zero dependencies.** Pure Rust compiled to WASM. No npm runtime deps.
- **Sub-millisecond.** Every query, every time.

## Limits

| Limit | Value |
|-------|-------|
| Max documents | 500 |
| Max grep results | 100 |
| File types | `.md` only |
| Encoding | UTF-8 only |

## License

MIT
