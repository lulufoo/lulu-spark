import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const indexHtml = readFileSync(new URL('../frontend/index.html', import.meta.url), 'utf8');
const mainJs = readFileSync(new URL('../frontend/js/main.js', import.meta.url), 'utf8');

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
