import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

/** Concatenate every owner file under services/agent/loop/ for source-scan contracts. */
export function readAgentLoopSource() {
  const dir = join(repoRoot, 'src-tauri/src/services/agent/loop');
  return readdirSync(dir)
    .filter((name) => name.endsWith('.rs'))
    .sort()
    .map((name) => readFileSync(join(dir, name), 'utf8'))
    .join('\n');
}
