#!/usr/bin/env node
/**
 * T5 independent reclassification ops runner (worktree Host harness).
 *
 * Live IDE MCP at ops time may lack `category_id` on update_todo_task; this
 * script exercises the same Host path (`set_master_category`) via cargo test
 * and writes reviewable evidence for the vitest gate.
 *
 * Usage (from repo root):
 *   node scripts/todo-reclassify-ops.mjs
 *
 * Optional:
 *   T5_OPS_EVIDENCE_OUT=scripts/fixtures/todo-reclassify-ops-evidence.json \
 *     node scripts/todo-reclassify-ops.mjs
 */
import { spawnSync } from 'node:child_process';
import { mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const defaultOut = join(
  repoRoot,
  'scripts/fixtures/todo-reclassify-ops-evidence.json',
);
const evidenceOut = resolve(process.env.T5_OPS_EVIDENCE_OUT || defaultOut);

mkdirSync(dirname(evidenceOut), { recursive: true });

const result = spawnSync(
  'cargo',
  [
    'test',
    '--lib',
    't5_independent_reclassify_ops_demo',
    '--',
    '--test-threads=1',
    '--nocapture',
  ],
  {
    cwd: join(repoRoot, 'src-tauri'),
    env: {
      ...process.env,
      TEST_MODE: '1',
      T5_OPS_EVIDENCE_OUT: evidenceOut,
    },
    encoding: 'utf8',
  },
);

process.stdout.write(result.stdout || '');
process.stderr.write(result.stderr || '');

if (result.status !== 0) {
  console.error('[todo-reclassify-ops] harness failed');
  process.exit(result.status ?? 1);
}

console.log(`[todo-reclassify-ops] evidence written: ${evidenceOut}`);
