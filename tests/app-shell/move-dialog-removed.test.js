import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { listFrontendSourceFiles } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const moveUiPath = join(repoRoot, 'frontend/src/app-shell/ui/move-dialog.tsx');
const shellPath = join(repoRoot, 'frontend/src/shell.tsx');
const notesMoveUiPath = join(repoRoot, 'frontend/src/notes/ui/move-project-dialog.tsx');
const notesMoveCommandPath = join(repoRoot, 'frontend/src/notes/commands/move-project-dialog.ts');

describe('Move document dialog removed', () => {
  it('deletes move-dialog.tsx', () => {
    expect(existsSync(moveUiPath)).toBe(false);
  });

  it('does not leave MoveDocDialog, openMoveDocDialog, closeMoveDocDialog, or re-exports', () => {
    const product = listFrontendSourceFiles(join(repoRoot, 'frontend/src'))
      .map((abs) => readFileSync(abs, 'utf8'))
      .join('\n');
    expect(product).not.toMatch(/\bMoveDocDialog\b/);
    expect(product).not.toMatch(/\bopenMoveDocDialog\b/);
    expect(product).not.toMatch(/\bcloseMoveDocDialog\b/);
    expect(product).not.toMatch(
      /export\s+(?:async\s+)?(?:function MoveDocDialog|const MoveDocDialog|class MoveDocDialog|type MoveDocDialog)\b/,
    );
    expect(product).not.toMatch(/export\s+\{[^}]*\bMoveDocDialog\b/);
    expect(product).not.toMatch(
      /export\s+(?:async\s+)?(?:function openMoveDocDialog|const openMoveDocDialog|class openMoveDocDialog|type openMoveDocDialog)\b/,
    );
    expect(product).not.toMatch(/export\s+\{[^}]*\bopenMoveDocDialog\b/);
    expect(product).not.toMatch(
      /export\s+(?:async\s+)?(?:function closeMoveDocDialog|const closeMoveDocDialog|class closeMoveDocDialog|type closeMoveDocDialog)\b/,
    );
    expect(product).not.toMatch(/export\s+\{[^}]*\bcloseMoveDocDialog\b/);
  });

  it('does not delete notes move-project-dialog', () => {
    expect(existsSync(notesMoveUiPath)).toBe(true);
    expect(existsSync(notesMoveCommandPath)).toBe(true);
    const ui = readFileSync(notesMoveUiPath, 'utf8');
    const commands = readFileSync(notesMoveCommandPath, 'utf8');
    expect(ui).toMatch(/\bMoveProjectDialog\b/);
    expect(ui).toMatch(/\bopenMoveProjectDialog\b/);
    expect(commands).toMatch(/\bopenMoveProjectDialog\b/);
  });

  it('does not leave a menu call to openMoveDocDialog', () => {
    expect(existsSync(shellPath)).toBe(true);
    const shell = readFileSync(shellPath, 'utf8');
    expect(shell).not.toMatch(/\bopenMoveDocDialog\b/);
    expect(shell).not.toMatch(/\bMoveDocDialog\b/);
    expect(shell).not.toContain('id="btn-move-doc-header"');
  });
});
