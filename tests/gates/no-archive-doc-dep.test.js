import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const FORBIDDEN = ['docs', 'archive'].join('/');
const SCAN_DIRS = ['tests', 'scripts', 'frontend/src', 'src-tauri/src'];

function listFiles(dir, out = []) {
  if (!existsSync(dir)) return out;
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) {
      if (name === 'node_modules' || name === 'target' || name === '.git') continue;
      listFiles(full, out);
    } else {
      out.push(full);
    }
  }
  return out;
}

describe('no official/test code depends on archived documents', () => {
  it('tests, scripts, frontend/src, and src-tauri/src do not read archive docs', () => {
    const offenders = [];
    for (const rel of SCAN_DIRS) {
      for (const abs of listFiles(join(repoRoot, rel))) {
        const text = readFileSync(abs, 'utf8');
        if (text.includes(FORBIDDEN)) {
          offenders.push(relative(repoRoot, abs));
        }
      }
    }
    expect(offenders, `must not mention ${FORBIDDEN}:\n${offenders.join('\n')}`).toEqual([]);
  });
});
