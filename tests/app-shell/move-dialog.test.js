import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { listFrontendSourceFiles, readShellHtml } from '../helpers/read-frontend-js.js';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const appShellUi = listFrontendSourceFiles(join(repoRoot, 'frontend/src/app-shell/ui'))
  .sort()
  .map((abs) => readFileSync(abs, 'utf8'))
  .join('\n');
const indexHtml = [readShellHtml(), appShellUi].join('\n');
const moveDialogJs = [
  readFileSync(new URL('../../frontend/src/app-shell/ui/move-dialog.tsx', import.meta.url), 'utf8'),
  readFileSync(new URL('../../frontend/src/app-shell/commands/move-dialog.ts', import.meta.url), 'utf8'),
].join('\n');

describe('gh-ops delete panel HTML', () => {
  it('has no arming button in delete panel actions', () => {
    expect(indexHtml).not.toContain('id="btn-delete-doc-arm"');
  });

  it('confirm delete button is enabled by default', () => {
    expect(indexHtml).toContain('id="btn-delete-doc-ok"');
    expect(indexHtml).not.toMatch(/id="btn-delete-doc-ok"[^>]*\bdisabled(?!=\{deleteBusy\})/);
  });

  it('delete actions only contain confirm delete button', () => {
    const actionsMatch = indexHtml.match(
      /<div className="gh-ops-panel-actions gh-ops-delete-actions">([\s\S]*?)<\/div>/
    );
    expect(actionsMatch).not.toBeNull();
    const buttons = [...actionsMatch[1].matchAll(/<button\b/g)];
    expect(buttons).toHaveLength(1);
    expect(actionsMatch[1]).toContain('id="btn-delete-doc-ok"');
  });
});

describe('gh-ops delete panel JS', () => {
  it('has no arming state machine symbols', () => {
    expect(moveDialogJs).not.toMatch(/\barmDeleteDoc\b/);
    expect(moveDialogJs).not.toMatch(/\bresetDeleteArm\b/);
    expect(moveDialogJs).not.toContain("getElementById('btn-delete-doc-arm')");
  });

  it('resetDeleteColumn enables confirm delete button', () => {
    expect(moveDialogJs).toMatch(/function resetDeleteColumn\(\)[\s\S]*?setDeleteBusy\(false\)/);
  });

  it('doDeleteDoc has no disabled guard and re-enables ok in finally', () => {
    expect(moveDialogJs).not.toMatch(/if \(okBtn\.disabled\) return/);
    expect(moveDialogJs).not.toMatch(/\bresetDeleteArm\(\)/);
    expect(moveDialogJs).toMatch(/async function onDelete\(\)[\s\S]*?finally[\s\S]*?setDeleteBusy\(false\)/);
  });

  it('delete-url Enter directly calls doDeleteDoc', () => {
    expect(moveDialogJs).toMatch(
      /id="delete-url"[\s\S]*?onKeyDown[\s\S]*?if \(e\.key === 'Enter'\) void onDelete\(\)/
    );
  });
});
