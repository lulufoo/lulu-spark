import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { readShellHtml } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const indexHtml = [
  readShellHtml(),
  readFileSync(join(repoRoot, 'frontend/src/notes/ui/sidebar.tsx'), 'utf8'),
  readFileSync(join(repoRoot, 'frontend/src/notes/ui/search.tsx'), 'utf8'),
  readFileSync(join(repoRoot, 'frontend/src/knowledge/ui/search.tsx'), 'utf8'),
].join('\n');
const indexRebuildTsx = readFileSync(
  join(repoRoot, 'frontend/src/app-shell/ui/index-rebuild.tsx'),
  'utf8',
);

function extractTagOuter(html, id) {
  const re = new RegExp(`<[^>]+id="${id}"[^>]*>[\\s\\S]*?</[^>]+>`, 'i');
  const match = html.match(re);
  expect(match, `missing element #${id}`).toBeTruthy();
  return match[0];
}

describe('P1 copy-switch shell (tech-doc T1 / index.html)', () => {
  it('sets document language to en', () => {
    expect(indexHtml).toMatch(/<html[^>]*\blang="en"/);
    expect(indexHtml).not.toMatch(/lang="zh-CN"/);
  });

  it('header brand and entry controls align with table A', () => {
    expect(extractTagOuter(indexHtml, 'btn-nav-home-title')).toContain('Lulu Workbench');
    expect(extractTagOuter(indexHtml, 'btn-nav-home')).toContain('← Home');
    expect(indexHtml).toMatch(/data-panel="knowledge">Knowledge</);
    expect(indexHtml).not.toContain('id="btn-repo-menu"');
    expect(indexHtml).not.toContain('id="repo-menu-wrap"');
  });

  it('header menus use table B labels', () => {
    expect(extractTagOuter(indexHtml, 'btn-sync-menu')).toContain('⇕ Sync');
    expect(extractTagOuter(indexHtml, 'btn-tools-menu')).toContain('⛓ Tools');
    expect(extractTagOuter(indexHtml, 'btn-settings')).toMatch(/Settings/);
    expect(extractTagOuter(indexHtml, 'btn-push-index')).toContain('↑ Commit changes');
    expect(extractTagOuter(indexHtml, 'btn-pull')).toContain('↓ Update project');
    expect(extractTagOuter(indexHtml, 'btn-local-refresh')).toContain('⟳ Refresh local');
  });

  it('search placeholders use table B; index rebuild lives only in the header', () => {
    expect(indexHtml).toMatch(/id="gs-wb-input"[^>]*placeholder="Search notes…"/);
    expect(indexHtml).toMatch(/id="gs-kb-input"[^>]*placeholder="Search knowledge…"/);
    expect(indexHtml).not.toMatch(/id="gs-wb-rebuild-btn"/);
    expect(indexHtml).not.toMatch(/id="gs-kb-rebuild-btn"/);
    expect(indexHtml).toMatch(/<IndexRebuildButton \/>/);
    expect(indexRebuildTsx).toMatch(/id="btn-index-rebuild"/);
    expect(indexRebuildTsx).toContain('Rebuild search index');
  });

  it('repo list title uses table B2 knowledge list label', () => {
    expect(indexHtml).toMatch(/data-tab="list"[^>]*>List</);
    expect(indexHtml).toContain('id="repo-list-title-group"');
    expect(indexHtml).not.toContain('☰ 沉淀知识库列表');
  });

  it('sidebar resizer aria-label uses table B', () => {
    expect(indexHtml).toMatch(/id="sidebar-resizer"[^>]*aria-label="Resize sidebar"/);
  });

  it('read-later dialog title uses table A brand name', () => {
    expect(indexHtml).toMatch(/<h3>Read Later<\/h3>/);
    expect(indexHtml).not.toContain('Read Later 待读');
  });

  it('settings nav uses Workbench, Notes, Knowledge, Assistant, then Sync', () => {
    expect(indexHtml).toMatch(/data-panel="workbench">Workbench</);
    expect(indexHtml).toMatch(/data-panel="notes">Notes</);
    expect(indexHtml).toMatch(/data-panel="knowledge">Knowledge</);
    expect(indexHtml).toMatch(/data-panel="llm">Assistant</);
    expect(indexHtml).not.toMatch(/data-panel="llm">Assistant \/ Engine</);
    expect(indexHtml).toMatch(/data-panel="github">Sync</);
    expect(indexHtml).not.toMatch(/data-panel="github">GitHub account</);
    expect(indexHtml).toMatch(/data-tab="hidden"[^>]*>Hidden files</);
    const knowledgeTabs = indexHtml.match(/id="settings-panel-knowledge"[\s\S]*?role="tablist">([\s\S]*?)<\/div>/)?.[1] ?? '';
    expect(knowledgeTabs.indexOf('data-tab="directory"')).toBeGreaterThan(-1);
    expect(knowledgeTabs.indexOf('data-tab="directory"')).toBeLessThan(
      knowledgeTabs.indexOf('data-tab="list"'),
    );
    expect(knowledgeTabs.indexOf('data-tab="list"')).toBeLessThan(
      knowledgeTabs.indexOf('data-tab="categories"'),
    );
    expect(knowledgeTabs.indexOf('data-tab="categories"')).toBeLessThan(
      knowledgeTabs.indexOf('data-tab="add"'),
    );
    expect(knowledgeTabs.indexOf('data-tab="add"')).toBeLessThan(
      knowledgeTabs.indexOf('data-tab="hidden"'),
    );
    const nav = indexHtml.match(/<nav id="settings-nav">([\s\S]*?)<\/nav>/)?.[1] ?? '';
    expect(nav.indexOf('data-panel="workbench"')).toBeGreaterThan(-1);
    expect(nav.indexOf('data-panel="workbench"')).toBeLessThan(
      nav.indexOf('data-panel="notes"'),
    );
    expect(nav.indexOf('data-panel="notes"')).toBeLessThan(
      nav.indexOf('data-panel="knowledge"'),
    );
    expect(nav.indexOf('data-panel="knowledge"')).toBeLessThan(
      nav.indexOf('data-panel="llm"'),
    );
    expect(nav.indexOf('data-panel="llm"')).toBeLessThan(
      nav.indexOf('data-panel="github"'),
    );
  });

  it('delete dialog copy uses table B2 strings', () => {
    expect(indexHtml).toMatch(/<h3>⚠️ Delete document<\/h3>/);
    expect(indexHtml).toMatch(/id="delete-dialog-desc"[^>]*>[\s\S]*Cannot be undone/);
    expect(indexHtml).toMatch(/placeholder="Enter CONFIRM here"/);
    expect(indexHtml).toMatch(/title="Copy CONFIRM"/);
  });

  it('comment editor placeholder uses table B2', () => {
    expect(indexHtml).toMatch(
      /data-placeholder="Comment… \(Ctrl\/Cmd\+Enter to save\)"/,
    );
  });
});
