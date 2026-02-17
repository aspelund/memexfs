export interface GrepResult {
  path: string;
  line: number;
  content: string;
}

export interface ToolDefinition {
  name: string;
  description: string;
  parameters: Record<string, { type: string; description: string }>;
  required: string[];
}

export interface PoolOptions {
  /** Number of worker threads. Defaults to `os.availableParallelism()`. */
  workers?: number;
}

export interface MemexPool {
  /** Search for a pattern across all documents. */
  grep(pattern: string, glob?: string): Promise<GrepResult[]>;

  /** Read a document's content, optionally with offset and line limit. */
  read(path: string, offset?: number, limit?: number): Promise<string>;

  /** List immediate children of a directory. */
  ls(path: string): Promise<string[]>;

  /** Return tool definitions for LLM integration. */
  toolDefinitions(): Promise<ToolDefinition[]>;

  /** Return the number of loaded documents. */
  documentCount(): Promise<number>;

  /** Terminate all workers. The pool cannot be used after this. */
  terminate(): Promise<void>;
}

/**
 * Create a pool of MemexFS worker threads.
 * Each worker loads its own WASM instance from the given directory.
 * Requests are dispatched to the least-busy worker.
 *
 * @param dir - Directory of .md files to load.
 * @param opts - Pool options.
 */
export function createPool(dir: string, opts?: PoolOptions): Promise<MemexPool>;
