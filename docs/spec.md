# API Specification

## Initialization

### `new MemexFS(path: string)`

Loads all `.md` files recursively from `path`, builds an inverted index, and returns a ready-to-query instance.

- **path**: Absolute or relative path to the document root directory
- **Throws**: If path does not exist or contains no `.md` files
- **Performance**: < 100ms for 500 documents

```js
const fs = new MemexFS("./docs");
```

### `MemexFS.fromBlob(blob: Uint8Array)`

Loads a pre-packed binary blob of documents. Used for production deployments where docs are bundled at build time.

```js
const fs = MemexFS.fromBlob(docsBlob);
```

---

## Operations

### `grep(pattern: string, glob?: string): GrepResult[]`

Search for a pattern across all indexed documents.

#### Parameters

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `pattern` | `string` | yes | Search pattern. Plain text for exact token match, or regex syntax for pattern matching. |
| `glob` | `string` | no | File path filter using glob syntax. Only documents matching this pattern are searched. |

#### Returns

```ts
interface GrepResult {
  path: string;     // relative path from document root
  line: number;     // 1-indexed line number
  content: string;  // the full matching line
}
```

Returns an empty array if no matches found. Results are ordered by file path, then line number.

#### Examples

```js
// Simple text search
fs.grep("password reset");
// [{ path: "account/password-reset.md", line: 3, content: "## How to reset your password" }]

// Regex search
fs.grep("price|cost|billing");

// Scoped to a folder
fs.grep("refund", "billing/**/*.md");
```

#### Behavior

- Simple patterns (no regex metacharacters) use the inverted index for O(1) lookup
- Regex patterns fall back to linear scan over all lines
- Glob filtering is applied before content search
- Pattern matching is case-insensitive by default
- Maximum results: 100 (to prevent flooding the LLM context)

---

### `read(path: string, options?: ReadOptions): string`

Read the contents of a document.

#### Parameters

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `string` | yes | Document path relative to the root |
| `options.offset` | `number` | no | Line number to start from (1-indexed). Defaults to 1. |
| `options.limit` | `number` | no | Number of lines to return. Defaults to all lines. |

#### Returns

The document content as a string with line numbers prepended:

```
  1  # Password Reset
  2
  3  ## How to reset your password
  4
  5  1. Go to Settings > Account
```

Line numbers are included so the LLM can reference specific lines and make informed `offset`/`limit` calls on follow-up reads.

#### Examples

```js
// Full document
fs.read("account/password-reset.md");

// Lines 10-30 only
fs.read("account/password-reset.md", { offset: 10, limit: 20 });
```

#### Behavior

- Returns the full document if no `offset`/`limit` provided
- `offset` is 1-indexed (line 1 = first line of file)
- If `offset` exceeds document length, returns empty string
- If `offset + limit` exceeds document length, returns lines until end of file
- **Throws** if path does not exist in the filesystem

---

## Convenience methods

### `toolDefinitions(): ToolDefinition[]`

Returns the two tool definitions as a JSON-serializable array, ready to pass to an LLM API.

```js
const tools = fs.toolDefinitions();
// Pass directly to Anthropic, OpenAI, etc.
```

### `call(name: string, params: object): string`

Dispatches a tool call by name. Designed for direct use in a tool-calling loop.

```js
// In your tool-calling loop:
const result = fs.call(toolUse.name, toolUse.input);
// result is always a string — grep returns JSON-stringified results, read returns text
```

| Call | Return type |
|------|------------|
| `call("grep", { pattern, glob? })` | JSON string of `GrepResult[]` |
| `call("read", { path, offset?, limit? })` | Document text with line numbers |

**Throws** if `name` is not `"grep"` or `"read"`.

---

## CLI

### `npx memexfs pack <dir> -o <output>`

Packs a directory of markdown files into a binary blob for production use.

```bash
npx memexfs pack ./docs -o docs.memex
```

### `npx memexfs stats <dir>`

Prints statistics about a document directory:

```bash
npx memexfs stats ./docs
# Documents: 342
# Total lines: 28,491
# Total size: 8.2 MB
# Unique tokens: 12,847
# Estimated memory: ~38 MB
```

### `npx memexfs bench <dir>`

Runs benchmarks comparing memexfs grep/read against system grep and file read:

```bash
npx memexfs bench ./docs
# grep "password reset"
#   system grep:  4.2ms
#   memexfs grep: 0.03ms (140x faster)
#
# read account/password-reset.md
#   system read:  0.8ms
#   memexfs read: 0.01ms (80x faster)
```

---

## Error handling

All errors are thrown as exceptions with descriptive messages:

| Error | When |
|-------|------|
| `MemexError: path not found: ./nope` | Init with nonexistent directory |
| `MemexError: no documents found in ./empty` | Init with directory containing no .md files |
| `MemexError: document not found: billing/nope.md` | Read with nonexistent path |
| `MemexError: invalid regex: [unclosed` | Grep with invalid regex pattern |
| `MemexError: unknown tool: delete` | Call with unsupported tool name |

---

## Limits

| Constraint | Value | Reason |
|------------|-------|--------|
| Max documents | 500 | Design constraint, not a hard limit |
| Max grep results | 100 | Prevent LLM context flooding |
| Max read lines | 2000 | Default limit, same as Claude Code's Read tool |
| File extensions | `.md` only | v1 scope |
| Encoding | UTF-8 only | Rust's native string type |
