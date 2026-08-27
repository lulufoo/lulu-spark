import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

/** Concatenate every `.rs` file in a directory (sorted), matching Host `read_rs_dir`. */
export function readRsDir(dir) {
  return readdirSync(dir)
    .filter((name) => name.endsWith('.rs'))
    .sort()
    .map((name) => readFileSync(join(dir, name), 'utf8'))
    .join('\n');
}

/** Read a `.rs` file, or concatenate a Rust module directory. */
export function readRsPath(absPath) {
  if (statSync(absPath).isDirectory()) return readRsDir(absPath);
  return readFileSync(absPath, 'utf8');
}
