import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const pkg = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8'));
const testScript = pkg.scripts.test;

const TODO_TASK_VITEST = [
  'tests/home-entry-shell/hub.test.js',
  'tests/gates/ai-assistant-p4-smoke.test.js',
  'tests/gates/todo-desktop-surface-removed.test.js',
];

const NOTE_FEATURE_VITEST = [
  'tests/notes/assistant.test.js',
  'tests/notes/viewer-create-note.test.js',
  'tests/notes/ac-gate.test.js',
];

describe('AC-全量回归 gate (tech-doc T-08 / VF)', () => {
  it('npm test includes remaining desktop/todo-removal vitest files', () => {
    expect(testScript).toMatch(/vitest run --dir tests/);
    for (const file of TODO_TASK_VITEST) {
      expect(existsSync(join(repoRoot, file)), `missing ${file}`).toBe(true);
    }
  });

  it('npm test includes note feature vitest files (tech-doc T-13 / VF)', () => {
    expect(testScript).toMatch(/vitest run --dir tests/);
    for (const file of NOTE_FEATURE_VITEST) {
      expect(existsSync(join(repoRoot, file)), `missing ${file}`).toBe(true);
    }
  });

  it('npm test runs full cargo test --lib single-threaded', () => {
    expect(testScript).toMatch(/cargo test --lib/);
    expect(testScript).toMatch(/--test-threads=1/);
    expect(testScript).not.toMatch(/TEST_MODE=1/);
    expect(testScript).toMatch(/cargo test --lib/);
    expect(testScript).toMatch(/--test-threads=1/);
  });

  it('npm test runs Host MCP verify-host-mcp.mjs', () => {
    expect(testScript).toContain('scripts/verify-host-mcp.mjs');
  });
});
