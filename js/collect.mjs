import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

/**
 * Recursively collect all .md files under a directory.
 * Returns [[relativePath, content], ...] sorted by path.
 */
export function collectMdFiles(rootDir, currentDir) {
  const entries = readdirSync(currentDir);
  const docs = [];

  for (const entry of entries) {
    const fullPath = join(currentDir, entry);
    const stat = statSync(fullPath);

    if (stat.isDirectory()) {
      docs.push(...collectMdFiles(rootDir, fullPath));
    } else if (entry.endsWith(".md")) {
      const relPath = relative(rootDir, fullPath);
      const content = readFileSync(fullPath, "utf-8");
      docs.push([relPath, content]);
    }
  }

  return docs;
}
