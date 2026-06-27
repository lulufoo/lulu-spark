import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const indexHtml = readFileSync(new URL('../frontend/index.html', import.meta.url), 'utf8');
const mainJs = readFileSync(new URL('../frontend/js/main.js', import.meta.url), 'utf8');
const apiJs = readFileSync(new URL('../frontend/js/api.js', import.meta.url), 'utf8');
const moveDialogJs = readFileSync(
  new URL('../frontend/js/components/modals/move-dialog.js', import.meta.url),
  'utf8'
);

describe('sediment kb UI shell', () => {
  it('menu renamed to 沉淀知识库 with add/manage/list entries', () => {
    expect(indexHtml).toContain('id="btn-repo-menu">⚙ 沉淀知识库</button>');
    expect(indexHtml).toContain('id="btn-sediment-kb-add"');
    expect(indexHtml).toContain('id="btn-sediment-kb-manage"');
    expect(indexHtml).toContain('id="btn-sediment-kb-list"');
    expect(indexHtml).not.toContain('id="btn-repo-list"');
  });

  it('removes full corpus sync button from list dialog', () => {
    expect(indexHtml).not.toContain('id="btn-kb-corpus-sync"');
  });

  it('add and manage dialogs exist', () => {
    expect(indexHtml).toContain('id="sediment-kb-add-dialog"');
    expect(indexHtml).toContain('id="sediment-kb-manage-dialog"');
    expect(indexHtml).toContain('id="sediment-kb-add-url"');
    expect(indexHtml).toContain('id="sediment-kb-add-category"');
    expect(indexHtml).toContain('id="sediment-kb-add-description"');
  });

  it('add dialog description field is optional textarea', () => {
    const match = indexHtml.match(/<textarea id="sediment-kb-add-description"[^>]*>/);
    expect(match).not.toBeNull();
  });

  it('openSedimentKbAddDialog clears description input', () => {
    expect(mainJs).toMatch(
      /function openSedimentKbAddDialog\(\)[\s\S]*?getElementById\('sediment-kb-add-description'\)\.value = ''/
    );
  });

  it('submit handler passes description to addSedimentKbRepo', () => {
    expect(mainJs).toMatch(
      /btn-sediment-kb-add-submit[\s\S]*?getElementById\('sediment-kb-add-description'\)[\s\S]*?api\.addSedimentKbRepo\([^)]*description/
    );
  });

  it('addSedimentKbRepo includes description in request body', () => {
    expect(apiJs).toMatch(/export async function addSedimentKbRepo\(fullName, categoryId, description\)/);
    expect(apiJs).toMatch(/body\.description = description/);
  });

  it('main.js wires sediment kb helpers', () => {
    expect(mainJs).toMatch(/function openSedimentKbAddDialog\(/);
    expect(mainJs).toMatch(/function openSedimentKbManageDialog\(/);
    expect(mainJs).toMatch(/function renderSedimentKbListByCategory\(/);
    expect(mainJs).toMatch(/function onInlineCategoryChange\(/);
    expect(mainJs).toMatch(/function onDeleteSedimentKbRepo\(/);
    expect(mainJs).not.toMatch(/btn-kb-corpus-sync/);
  });
});

describe('gh-ops delete panel HTML', () => {
  it('has no arming button in delete panel actions', () => {
    expect(indexHtml).not.toContain('id="btn-delete-doc-arm"');
  });

  it('confirm delete button is enabled by default', () => {
    const match = indexHtml.match(/<button id="btn-delete-doc-ok"[^>]*>/);
    expect(match).not.toBeNull();
    expect(match[0]).not.toMatch(/\bdisabled\b/);
  });

  it('delete actions only contain confirm delete button', () => {
    const actionsMatch = indexHtml.match(
      /<div class="gh-ops-panel-actions gh-ops-delete-actions">([\s\S]*?)<\/div>/
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
    expect(moveDialogJs).toMatch(/function resetDeleteColumn\(\)[\s\S]*?deleteOkBtn\.disabled = false/);
  });

  it('doDeleteDoc has no disabled guard and re-enables ok in finally', () => {
    expect(moveDialogJs).not.toMatch(/if \(okBtn\.disabled\) return/);
    expect(moveDialogJs).not.toMatch(/\bresetDeleteArm\(\)/);
    expect(moveDialogJs).toMatch(/async function doDeleteDoc\(\)[\s\S]*?finally[\s\S]*?okBtn\.disabled = false/);
  });

  it('delete-url Enter directly calls doDeleteDoc', () => {
    expect(moveDialogJs).toMatch(
      /getElementById\('delete-url'\)\.addEventListener\('keydown'[\s\S]*?if \(e\.key === 'Enter'\) doDeleteDoc\(\)/
    );
  });
});
