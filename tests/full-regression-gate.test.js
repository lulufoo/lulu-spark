import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const pkg = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8'));
const testScript = pkg.scripts.test;

const PLAN_TASK_VITEST = [
  'tests/home-hub.test.js',
  'tests/plan-task-assistant.test.js',
  'tests/plan-task-split.test.js',
];

describe('AC-全量回归 gate (tech-doc T-08 / VF)', () => {
  it('npm test includes plan-task feature vitest files', () => {
    for (const file of PLAN_TASK_VITEST) {
      expect(testScript, `missing ${file} in npm test`).toContain(file);
    }
  });

  it('npm test runs full cargo test --lib with TEST_MODE=1 single-threaded', () => {
    expect(testScript).toMatch(/TEST_MODE=1/);
    expect(testScript).toMatch(/cargo test --lib/);
    expect(testScript).toMatch(/--test-threads=1/);
  });

  it('npm test runs knowledge-mcp verify.mjs', () => {
    expect(testScript).toContain('packages/knowledge-mcp/scripts/verify.mjs');
  });
});
