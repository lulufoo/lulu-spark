import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { readShellHtml } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const shellSrc = readFileSync(join(repoRoot, 'frontend/src/shell.tsx'), 'utf8');
const homeSrc = readFileSync(join(repoRoot, 'frontend/src/home/page.tsx'), 'utf8');
const indexHtml = [
  readShellHtml(),
  homeSrc,
  readFileSync(join(repoRoot, 'frontend/src/shared/home-mark.tsx'), 'utf8'),
  readFileSync(join(repoRoot, 'frontend/src/notes/ui/sidebar.tsx'), 'utf8'),
  readFileSync(join(repoRoot, 'frontend/src/notes/ui/search.tsx'), 'utf8'),
  readFileSync(join(repoRoot, 'frontend/src/knowledge/ui/search.tsx'), 'utf8'),
].join('\n');

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

  it('home brand and entry controls align with table A', () => {
    expect(homeSrc).not.toMatch(/home-chat-brand-label/);
    expect(homeSrc).not.toMatch(/<HomeMark/);
    expect(homeSrc).toMatch(/<WindowDragStrip/);
    expect(homeSrc).toMatch(/data-tauri-drag-region/);
    expect(indexHtml).not.toMatch(/id="btn-nav-home-title"/);
    expect(indexHtml).not.toMatch(/id="btn-nav-home"(?!-)/);
    expect(indexHtml).toMatch(/data-panel="knowledge">Knowledge</);
    expect(indexHtml).not.toContain('id="btn-repo-menu"');
    expect(indexHtml).not.toContain('id="repo-menu-wrap"');
  });

  it('home workspace rail uses table B labels', () => {
    expect(indexHtml).not.toContain('id="btn-sync-menu"');
    expect(indexHtml).not.toContain('⇕ Sync');
    expect(indexHtml).not.toContain('id="btn-push-index"');
    expect(indexHtml).not.toContain('id="btn-pull"');
    expect(indexHtml).not.toContain('id="btn-local-refresh"');
    expect(indexHtml).not.toContain('id="btn-tools-menu"');
    expect(homeSrc).toMatch(/data-home-nav-group="workspace"/);
    expect(homeSrc).toMatch(/data-home-nav-group="settings"/);
    expect(homeSrc).toMatch(/home-chat-nav-label">Workspace</);
    expect(homeSrc).toMatch(/home-chat-nav-label">Settings</);
    expect(homeSrc).not.toMatch(/home-chat-nav-label">Library</);
    expect(homeSrc).toMatch(/id="btn-bind"[\s\S]*?Bind Device/);
    expect(extractTagOuter(indexHtml, 'btn-settings')).toMatch(/Settings/);
  });

  it('keeps Bind on the home rail and unmounts GitHub and Convert', () => {
    expect(shellSrc).not.toMatch(/id="btn-tools-menu"/);
    expect(shellSrc).toMatch(/<BindDialog\s*\/>/);
    expect(homeSrc).toMatch(/id="btn-bind"/);
    expect(homeSrc).toMatch(/id="btn-bind"[\s\S]{0,280}openBindDialog/);
    expect(homeSrc).toMatch(/id="btn-settings"/);
    expect(shellSrc).not.toContain('id="btn-move-doc-header"');
    expect(shellSrc).not.toContain('id="btn-convert"');
    expect(shellSrc).not.toMatch(/\bConvertDialog\b/);
    expect(shellSrc).not.toMatch(/\bMoveDocDialog\b/);
    expect(shellSrc).not.toMatch(/\bopenConvertDialog\b/);
    expect(shellSrc).not.toMatch(/\bopenMoveDocDialog\b/);
  });

  it('search placeholders use table B; the header has no index rebuild control', () => {
    expect(indexHtml).toMatch(/id="gs-wb-input"[^>]*placeholder="Search notes…"/);
    expect(indexHtml).toMatch(/id="gs-kb-input"[^>]*placeholder="Search knowledge…"/);
    expect(indexHtml).not.toMatch(/id="gs-wb-rebuild-btn"/);
    expect(indexHtml).not.toMatch(/id="gs-kb-rebuild-btn"/);
    expect(indexHtml).not.toMatch(/IndexRebuildStatus|IndexRebuildButton|header-index-status|btn-index-rebuild/);
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

  it('settings nav uses Notes, Knowledge, Agent, then MCP', () => {
    expect(indexHtml).not.toMatch(/data-panel="spark">Data</);
    expect(indexHtml).toMatch(/data-panel="notes">Notes</);
    expect(indexHtml).toMatch(/data-panel="knowledge">Knowledge</);
    expect(indexHtml).toMatch(/data-panel="llm">Agent</);
    expect(indexHtml).not.toMatch(/data-panel="llm">Assistant/);
    expect(indexHtml).not.toMatch(/data-panel="github">Sync</);
    expect(indexHtml).not.toMatch(/data-panel="github">GitHub account</);
    expect(indexHtml).toMatch(/data-panel="mcp">MCP</);
    expect(indexHtml).toMatch(/data-tab="hidden"[^>]*>Hidden files</);
    const knowledgeTabs = indexHtml.match(/id="settings-panel-knowledge"[\s\S]*?role="tablist">([\s\S]*?)<\/div>/)?.[1] ?? '';
    expect(knowledgeTabs.indexOf('data-tab="directory"')).toBeGreaterThan(-1);
    expect(knowledgeTabs.indexOf('data-tab="directory"')).toBeLessThan(
      knowledgeTabs.indexOf('data-tab="categories"'),
    );
    expect(knowledgeTabs.indexOf('data-tab="categories"')).toBeLessThan(
      knowledgeTabs.indexOf('data-tab="list"'),
    );
    expect(knowledgeTabs.indexOf('data-tab="list"')).toBeLessThan(
      knowledgeTabs.indexOf('data-tab="add"'),
    );
    expect(knowledgeTabs.indexOf('data-tab="add"')).toBeLessThan(
      knowledgeTabs.indexOf('data-tab="hidden"'),
    );
    const nav = indexHtml.match(/<nav id="settings-nav">([\s\S]*?)<\/nav>/)?.[1] ?? '';
    expect(nav).not.toMatch(/data-panel="spark"/);
    expect(nav.indexOf('data-panel="notes"')).toBeGreaterThan(-1);
    expect(nav.indexOf('data-panel="notes"')).toBeLessThan(
      nav.indexOf('data-panel="knowledge"'),
    );
    expect(nav.indexOf('data-panel="knowledge"')).toBeLessThan(
      nav.indexOf('data-panel="llm"'),
    );
    expect(nav.indexOf('data-panel="llm"')).toBeLessThan(
      nav.indexOf('data-panel="mcp"'),
    );
    expect(nav).not.toMatch(/data-panel="github"/);
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
