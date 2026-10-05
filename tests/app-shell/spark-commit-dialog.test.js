import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

const deleted = [
  'frontend/src/app-shell/ui/spark-commit-dialog.tsx',
  'frontend/src/app-shell/commands/spark-commit-dialog.ts',
  'frontend/src/app-shell/state/spark-commit.ts',
];

describe('SparkCommitDialog retired', () => {
  it('removes the orphan commit dialog module', () => {
    for (const rel of deleted) {
      expect(existsSync(join(repoRoot, rel)), rel).toBe(false);
    }
    const shell = readFileSync(join(repoRoot, 'frontend/src/shell.tsx'), 'utf8');
    const boot = readFileSync(join(repoRoot, 'frontend/src/boot.ts'), 'utf8');
    expect(shell).not.toMatch(/SparkCommitDialog/);
    expect(boot).not.toMatch(/spark-commit-dialog/);
  });
});
