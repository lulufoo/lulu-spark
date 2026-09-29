import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

describe('gh-ops delete panel HTML', () => {
  it('deletes the move document dialog module', () => {
    expect(existsSync(join(repoRoot, 'frontend/src/app-shell/ui/move-dialog.tsx'))).toBe(false);
  });
});

describe('gh-ops delete panel JS', () => {
  it('deletes the move document command module', () => {
    expect(existsSync(join(repoRoot, 'frontend/src/app-shell/commands/move-dialog.ts'))).toBe(false);
  });
});
