import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const moveUiPath = join(repoRoot, 'frontend/src/app-shell/ui/move-dialog.tsx');
const moveCommandPath = join(repoRoot, 'frontend/src/app-shell/commands/move-dialog.ts');

describe('gh-ops delete panel HTML', () => {
  it('deletes the move document dialog module', () => {
    expect(existsSync(moveUiPath)).toBe(false);
  });
});

describe('gh-ops delete panel JS', () => {
  it('has no arming state machine symbols', () => {
    expect(existsSync(moveCommandPath)).toBe(true);
    const moveDialogJs = readFileSync(moveCommandPath, 'utf8');
    expect(moveDialogJs).not.toMatch(/\barmDeleteDoc\b/);
    expect(moveDialogJs).not.toMatch(/\bresetDeleteArm\b/);
    expect(moveDialogJs).not.toContain("getElementById('btn-delete-doc-arm')");
  });

  it('doDeleteDoc has no disabled guard', () => {
    const moveDialogJs = readFileSync(moveCommandPath, 'utf8');
    expect(moveDialogJs).toMatch(/export async function doDeleteDoc/);
    expect(moveDialogJs).not.toMatch(/if \(okBtn\.disabled\) return/);
    expect(moveDialogJs).not.toMatch(/\bresetDeleteArm\(\)/);
  });
});
