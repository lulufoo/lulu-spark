import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

function collectRsFiles(dir, out) {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) {
      collectRsFiles(path, out);
    } else if (name.endsWith('.rs')) {
      out.push(path);
    }
  }
}

/** Concatenate every `.rs` file under a directory tree (sorted), matching Host `read_rs_dir`. */
export function readRsDir(dir) {
  const files = [];
  collectRsFiles(dir, files);
  return files
    .sort()
    .map((path) => readFileSync(path, 'utf8'))
    .join('\n');
}

/** Read a `.rs` file, or concatenate a Rust module directory tree. */
export function readRsPath(absPath) {
  if (statSync(absPath).isDirectory()) return readRsDir(absPath);
  return readFileSync(absPath, 'utf8');
}
