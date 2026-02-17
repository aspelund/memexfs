import { MemexFS } from "../pkg/memexfs.js";

/**
 * Load all .md files recursively from a directory into a MemexFS instance.
 * @param dir - Absolute or relative path to the directory.
 */
export function loadFromDirectory(dir: string): MemexFS;
