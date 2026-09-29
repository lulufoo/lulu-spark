import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

const leftoverTestFiles = [
  'tests/app-shell/convert-qr.test.js',
  'tests/app-shell/qr-dialog.test.js',
  'tests/app-shell/move-dialog.test.js',
];

const deletedProductModules = [
  'frontend/src/app-shell/ui/move-dialog.tsx',
  'frontend/src/app-shell/commands/move-dialog.ts',
  'frontend/src/app-shell/ui/convert-dialog.tsx',
  'frontend/src/app-shell/commands/convert-dialog.ts',
  'frontend/src/app-shell/ui/qr-dialog.tsx',
  'frontend/src/app-shell/commands/qr-dialog.ts',
];

const t6Path = join(repoRoot, 'tests/gates/copy-switch-t6-modals.test.js');
const t1Path = join(repoRoot, 'tests/gates/copy-switch-t1-shell.test.js');
const bindTestPath = join(repoRoot, 'tests/app-shell/bind-dialog.test.js');
const apiTestPath = join(repoRoot, 'tests/host/api.test.js');
const syncMapTestPath = join(repoRoot, 'tests/host/syncApiInvokeMap.test.js');

const REACHABLE_SYMBOL_RE =
  /gh_move_assets|openConvertDialog|openMoveDocDialog|btn-convert|btn-move-doc-header/;

/** Absolute files under `dir` (or the file itself). */
function listFiles(dir, out = []) {
  if (!existsSync(dir)) return out;
  if (statSync(dir).isFile()) {
    out.push(dir);
    return out;
  }
  for (const name of readdirSync(dir)) {
    const abs = join(dir, name);
    if (statSync(abs).isDirectory()) listFiles(abs, out);
    else out.push(abs);
  }
  return out;
}

/** Paths under the rg roots that still mention a deleted menu symbol. */
function reachableHits() {
  const roots = [
    'frontend/src',
    'src-tauri/src',
    'src-tauri/permissions/sync-api.toml',
    'src-tauri/gen/schemas/acl-manifests.json',
  ];
  const hits = [];
  for (const root of roots) {
    for (const abs of listFiles(join(repoRoot, root))) {
      if (REACHABLE_SYMBOL_RE.test(readFileSync(abs, 'utf8'))) {
        hits.push(abs.slice(repoRoot.length + 1));
      }
    }
  }
  return hits;
}

describe('leftover Convert / qr / move tests rewritten', () => {
  it('deletes convert-qr, qr-dialog, and move-dialog test files', () => {
    for (const rel of leftoverTestFiles) {
      expect(existsSync(join(repoRoot, rel)), rel).toBe(false);
    }
  });

  it('stops t6-modals from reading deleted dialogs and keeps notes move-project-dialog', () => {
    expect(existsSync(t6Path)).toBe(true);
    const t6 = readFileSync(t6Path, 'utf8');
    for (const rel of deletedProductModules) {
      expect(t6, rel).not.toContain(rel);
    }
    expect(t6).not.toMatch(/move-dialog\.js uses table B\/B2 move\/delete copy/);
    expect(t6).toContain('frontend/src/notes/ui/move-project-dialog.tsx');
    expect(t6).toContain('frontend/src/notes/commands/move-project-dialog.ts');
    expect(t6).toContain('frontend/src/app-shell/ui/workbench-commit-dialog.tsx');
    expect(t6).toContain('frontend/src/notes/ui/delete-dialog.tsx');
    expect(t6).toContain('frontend/src/notes/ui/settle-dialog.tsx');
  });

  it('keeps host api tests and drops ghMove / gh-move cases', () => {
    expect(existsSync(apiTestPath)).toBe(true);
    expect(existsSync(syncMapTestPath)).toBe(true);
    const apiTest = readFileSync(apiTestPath, 'utf8');
    const syncTest = readFileSync(syncMapTestPath, 'utf8');
    expect(apiTest).not.toMatch(/\bghMove\b/);
    expect(apiTest).not.toMatch(/['"]\/api\/gh-move['"]/);
    expect(syncTest).not.toMatch(/\bghMove\b/);
    expect(syncTest).not.toMatch(/['"]\/api\/gh-move['"]/);
    expect(apiTest).toMatch(/\bghDelete\b/);
    expect(syncTest).toMatch(/['"]\/api\/gh-delete['"]/);
  });

  it('rewrites the tools-menu copy assertion to Bind', () => {
    expect(existsSync(t1Path)).toBe(true);
    const t1 = readFileSync(t1Path, 'utf8');
    expect(t1).toMatch(/id="btn-tools-menu"[^>]*>\s*Bind\s*</);
    expect(t1).not.toMatch(/⛓ Tools/);
    expect(t1).toMatch(/id="btn-bind"/);
    expect(t1).toMatch(/openBindDialog/);
  });

  it('keeps bind-dialog.test.js as Bind-only coverage', () => {
    expect(existsSync(bindTestPath)).toBe(true);
    const bindTest = readFileSync(bindTestPath, 'utf8');
    expect(bindTest).toMatch(/openBindDialog/);
    expect(bindTest).toMatch(/id="btn-bind"/);
    expect(bindTest).toMatch(/id="bind-dialog"/);
    expect(bindTest).not.toContain('qr-dialog.tsx');
    expect(bindTest).not.toContain('qr-dialog.ts');
    expect(bindTest).not.toContain('convert-dialog.tsx');
    expect(bindTest).not.toContain('convert-dialog.ts');
    expect(bindTest).not.toContain('base64-input');
    expect(bindTest).not.toMatch(/makeEl\(\s*['"]convert-dialog['"]\s*\)/);
    expect(bindTest).not.toMatch(/makeEl\(\s*['"]qr-preview['"]\s*\)/);
    expect(bindTest).not.toMatch(
      /export\s+(?:async\s+)?(?:function openConvertDialog|const openConvertDialog|class openConvertDialog|type openConvertDialog)\b/,
    );
    expect(bindTest).not.toMatch(/export\s+\{[^}]*\bopenConvertDialog\b/);
  });

  it('does not restore deleted Convert / qr / move product modules', () => {
    for (const rel of deletedProductModules) {
      expect(existsSync(join(repoRoot, rel)), rel).toBe(false);
    }
    expect(existsSync(join(repoRoot, 'frontend/src/app-shell/state/convert.ts'))).toBe(false);
  });

  it('cannot reach GitHub move or Convert from product surfaces', () => {
    expect(reachableHits()).toEqual([]);
    const boot = readFileSync(join(repoRoot, 'frontend/src/boot.ts'), 'utf8');
    expect(boot).not.toMatch(/app-shell\/ui\/qr-dialog/);
  });

  it('leaves Bind, notes move-project-dialog, host tests, and shared vendor scripts', () => {
    expect(existsSync(bindTestPath)).toBe(true);
    expect(existsSync(apiTestPath)).toBe(true);
    expect(existsSync(syncMapTestPath)).toBe(true);
    expect(existsSync(join(repoRoot, 'frontend/src/notes/ui/move-project-dialog.tsx'))).toBe(true);
    expect(existsSync(join(repoRoot, 'frontend/src/notes/commands/move-project-dialog.ts'))).toBe(true);
    expect(existsSync(join(repoRoot, 'frontend/vendor/qrcode.min.js'))).toBe(true);
    expect(existsSync(join(repoRoot, 'frontend/vendor/mermaid.min.js'))).toBe(true);
    expect(existsSync(join(repoRoot, 'frontend/src/app-shell/ui/bind-dialog.tsx'))).toBe(true);
    expect(existsSync(join(repoRoot, 'frontend/src/app-shell/commands/bind-dialog.ts'))).toBe(true);
    const bindUi = readFileSync(join(repoRoot, 'frontend/src/app-shell/ui/bind-dialog.tsx'), 'utf8');
    const bindCmd = readFileSync(join(repoRoot, 'frontend/src/app-shell/commands/bind-dialog.ts'), 'utf8');
    const bindRs = readFileSync(join(repoRoot, 'src-tauri/src/commands/bind.rs'), 'utf8');
    expect(bindUi).toMatch(
      /export\s+(?:async\s+)?(?:function BindDialog|const BindDialog|class BindDialog|type BindDialog)\b/,
    );
    expect(bindCmd).toMatch(
      /export\s+(?:async\s+)?(?:function openBindDialog|const openBindDialog|class openBindDialog|type openBindDialog)\b/,
    );
    expect(bindRs).toMatch(
      /(?:pub\s+)?(?:async\s+)?(?:fn issue_bind|const issue_bind|struct issue_bind|type issue_bind)\b/,
    );
    expect(bindRs).toMatch(
      /(?:pub\s+)?(?:async\s+)?(?:fn read_bind_session|const read_bind_session|struct read_bind_session|type read_bind_session)\b/,
    );
    expect(existsSync(join(repoRoot, 'src-tauri/src/unit-tests/commands/bind.rs'))).toBe(true);
  });
});
