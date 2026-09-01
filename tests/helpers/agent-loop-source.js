import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function readRsTree(dir) {
  return readdirSync(dir)
    .filter((name) => name.endsWith('.rs'))
    .sort()
    .map((name) => readFileSync(join(dir, name), 'utf8'))
    .join('\n');
}

/** Concatenate Host agent loop owner sources (binding / shell / turn + thin facade). */
export function readAgentLoopSource() {
  const agent = join(repoRoot, 'src-tauri/src/services/agent');
  return [
    readFileSync(join(agent, 'loop/mod.rs'), 'utf8'),
    readRsTree(join(agent, 'binding')),
    readRsTree(join(agent, 'shell')),
    readRsTree(join(agent, 'turn')),
  ].join('\n');
}

/** Concatenate Loop unit tests under unit-tests/services/agent/loop_tests/. */
export function readAgentLoopTestsSource() {
  const dir = join(repoRoot, 'src-tauri/src/unit-tests/services/agent/loop_tests');
  return readdirSync(dir)
    .filter((name) => name.endsWith('.rs'))
    .sort()
    .map((name) => readFileSync(join(dir, name), 'utf8'))
    .join('\n');
}
