/**
 * Recursively collect all .md files under a directory.
 * @param rootDir - The root directory (used to compute relative paths).
 * @param currentDir - The directory currently being scanned.
 * @returns An array of `[relativePath, content]` tuples.
 */
export function collectMdFiles(
  rootDir: string,
  currentDir: string,
): [path: string, content: string][];
