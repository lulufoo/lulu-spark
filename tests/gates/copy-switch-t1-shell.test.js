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
  readFileSync(join(repoRoot, 'frontend/src/notes/page.tsx'), 'utf8'),
  readFileSync(join(repoRoot, 'frontend/src/notes/ui/sidebar.tsx'), 'utf8'),
  readFileSync(join(repoRoot, 'frontend/src/notes/ui/search.tsx'), 'utf8'),
  readFileSync(join(repoRoot, 'frontend/src/knowledge/ui/sidebar.tsx'), 'utf8'),
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
    const css = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');
    expect(css).toMatch(/\.home-chat-sidebar \{[\s\S]*?--home-rail: 16px;/);
    expect(css).toMatch(/\.home-chat-sidebar \{[\s\S]*?--home-chat-row-inset: 8px;/);
    expect(css).toMatch(/\.home-chat-nav \{[\s\S]*?padding: 8px var\(--home-chat-row-inset\) 12px 0;/);
    expect(css).toMatch(/\.home-chat-nav-label \{[\s\S]*?padding: 6px var\(--home-rail\) 2px;/);
    expect(css).toMatch(/\.home-chat-nav-item \{[\s\S]*?padding: 7px 8px 7px var\(--home-rail\);/);
    expect(css).toMatch(
      /\.home-chat-sessions-head \{[\s\S]*?padding: 14px var\(--home-chat-row-inset\) 8px var\(--home-rail\);/,
    );
    expect(css).toMatch(/\.home-chat-sessions \{[\s\S]*?padding: 2px var\(--home-chat-row-inset\) 16px;/);
    const llmMark = readFileSync(join(repoRoot, 'frontend/src/shared/llm-mark.tsx'), 'utf8');
    expect(llmMark).toMatch(/translate\(2 2\) scale\(1\.17647\) translate\(-3\.5 -3\.5\)/);
    expect(css).toMatch(/\.home-chat-session\.is-active \{\s*background: #fff;\s*\}/);
    expect(css).toMatch(/\.home-chat-session-open \{[\s\S]*?padding: 8px;/);
    expect(css).toMatch(/\.home-chat-session-icon \{[\s\S]*?width: 24px;/);
    const sessionList = readFileSync(join(repoRoot, 'frontend/src/home/ui/session-list.tsx'), 'utf8');
    expect(sessionList).toMatch(/function SessionLlmIcon/);
    expect(sessionList).toMatch(/<ChatIcon \/>/);
    expect(sessionList).toMatch(/<LlmMark category=\{String\(llm\)\} \/>/);
    expect(sessionList).toMatch(/className="home-chat-session-label"/);
    expect(indexHtml).toMatch(/<WindowDragStrip/);
    expect(indexHtml).toMatch(/<PageBackHome/);
    expect(indexHtml).not.toMatch(/id="btn-nav-home-title"/);
    expect(indexHtml).not.toMatch(/id="btn-nav-home"(?!-)/);
    expect(indexHtml).toMatch(/data-panel="knowledge">Knowledge</);
    expect(indexHtml).not.toContain('id="btn-repo-menu"');
    expect(indexHtml).not.toContain('id="repo-menu-wrap"');
  });

  it('notes list puts search and date heading on one unboxed toolbar row', () => {
    const notesPage = readFileSync(join(repoRoot, 'frontend/src/notes/page.tsx'), 'utf8');
    const css = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');
    expect(notesPage).toMatch(
      /className="page-toolbar notes-page-toolbar"[\s\S]*hidden=\{showOutlet\}[\s\S]*<NotesDateHeading text=\{headingText\} \/>[\s\S]*<SparkSearch \/>/,
    );
    expect(notesPage).toMatch(/className="page-toolbar notes-page-toolbar"/);
    expect(notesPage).toMatch(/hidden=\{showOutlet\}/);
    expect(notesPage).not.toMatch(/<NotesStatus \/>\s*<NotesDateHeading \/>/);
    expect(css).toMatch(/\.page-back-home \{[\s\S]*?border: none;/);
    expect(css).toMatch(/\.notes-page-toolbar #date-heading \{/);
    expect(css).toMatch(/\.page-toolbar\.notes-page-toolbar \.gs-search-wrap \{[\s\S]*?margin-left: auto;/);
    expect(css).toMatch(/\.page-toolbar\.notes-page-toolbar \.gs-search-input[\s\S]*?width: 373px;/);
    expect(css).toMatch(/\.gs-search-input \{[\s\S]*?background: #fff;/);
    expect(css).toMatch(/\.page-toolbar \.gs-search-input \{[\s\S]*?background: #fff;/);
    expect(css).toMatch(/\.kb-reader-header \.gs-search-input[\s\S]*?background: #f6f8fa;/);
    expect(css).toMatch(/\.gs-search-input \{[\s\S]*?min-height: 36px;/);
    expect(css).not.toMatch(/\.gs-search-input \{[^}]*background: #525960;/);
    expect(css).toMatch(/\.notes-page-toolbar \.gs-search-dropdown \{[\s\S]*?left: auto;[\s\S]*?right: 0;/);
    expect(css).toMatch(/\.viewer-header \{[\s\S]*?padding: 12px 20px;/);
    expect(css).toMatch(/#md-header\.viewer-header \{[\s\S]*?padding: 20px 24px 16px;/);
    expect(css).not.toMatch(/\.viewer-panel-title \{\s*display:\s*none/);
    const notesSidebar = readFileSync(join(repoRoot, 'frontend/src/notes/ui/sidebar.tsx'), 'utf8');
    const knowledgeSidebar = readFileSync(join(repoRoot, 'frontend/src/knowledge/ui/sidebar.tsx'), 'utf8');
    expect(notesSidebar).toMatch(/<WindowDragStrip>\s*<PageBackHome \/>\s*<\/WindowDragStrip>/);
    expect(knowledgeSidebar).toMatch(/<WindowDragStrip>\s*<PageBackHome \/>\s*<\/WindowDragStrip>/);
    expect(knowledgeSidebar).not.toMatch(/knowledge-page-toolbar/);
    expect(knowledgeSidebar).not.toMatch(/page-toolbar-title/);
    expect(css).toMatch(/\.kb-reader-header\.viewer-header \{[\s\S]*?padding-left: 24px;/);
    expect(css).toMatch(/\.kb-reader-header\.viewer-header \{[\s\S]*?overflow: visible;/);
    expect(css).toMatch(/\.kb-reader \{[\s\S]*?overflow: visible;/);
    expect(css).toMatch(/\.knowledge-doc-reader-pane \{[\s\S]*?overflow: visible;/);
    expect(css).toMatch(/\.viewer-header-actions \{[\s\S]*?flex-shrink: 0;/);
    expect(css).toMatch(/\.kb-reader-header \.gs-search-wrap \{[\s\S]*?max-width: 373px;/);
    expect(css).toMatch(/\.kb-reader-header \.gs-search-wrap \{[\s\S]*?min-width: 0;/);
    expect(css).toMatch(/\.kb-reader-header \.gs-search-dropdown \{[\s\S]*?left: 0;/);
    expect(css).toMatch(/\.kb-reader-header\.viewer-header \{[\s\S]*?-webkit-app-region: drag;/);
    expect(css).toMatch(
      /\.kb-reader-header-actions,[\s\S]*?\.kb-reader-header \.gs-search-dropdown \{[\s\S]*?-webkit-app-region: no-drag;/,
    );
    const knowledgeShell = readFileSync(join(repoRoot, 'frontend/src/knowledge/ui/viewer/shell.tsx'), 'utf8');
    expect(knowledgeShell).toMatch(/class="kb-reader-header viewer-header" data-tauri-drag-region="deep"/);
    expect(knowledgeShell).toMatch(/className="kb-reader-header viewer-header" data-tauri-drag-region="deep"/);
    expect(knowledgeShell).toMatch(/<KnowledgeSearch hidden=\{false\} \/>/);
    expect(knowledgeShell).toMatch(/className="viewer-header-actions kb-reader-header-actions"/);
    expect(knowledgeShell).toMatch(/kb-btn-add-comment/);
    expect(notesPage).toMatch(/className="viewer-header-actions"/);
    expect(notesPage).toMatch(/id="btn-add-comment"/);
    expect(notesPage).toMatch(/💬 Comment/);
    const notesLinks = readFileSync(join(repoRoot, 'frontend/src/notes/ui/links-bar.tsx'), 'utf8');
    const kbLinks = readFileSync(join(repoRoot, 'frontend/src/knowledge/ui/links-bar.tsx'), 'utf8');
    const notesLinkCmd = readFileSync(join(repoRoot, 'frontend/src/notes/commands/links-bar.ts'), 'utf8');
    const kbLinkCmd = readFileSync(join(repoRoot, 'frontend/src/knowledge/commands/links-bar.ts'), 'utf8');
    expect(notesLinks).not.toMatch(/Add link/);
    expect(kbLinks).not.toMatch(/Add link/);
    expect(notesLinks).not.toMatch(/add-link/);
    expect(kbLinks).not.toMatch(/kb-link-add/);
    expect(notesLinkCmd).not.toMatch(/export async function addNoteLink/);
    expect(kbLinkCmd).not.toMatch(/export async function addKbLink/);
    expect(css).not.toMatch(/kb-btn-tree-toggle/);
    expect(css).not.toMatch(/is-tree-collapsed/);
    const homeMark = readFileSync(join(repoRoot, 'frontend/src/shared/home-mark.tsx'), 'utf8');
    expect(homeMark).toMatch(/aria-label="Home"/);
    expect(homeMark).not.toMatch(/page-back-home-label/);
    expect(homeMark).not.toMatch(/>Home</);
    expect(css).toMatch(/\.window-drag-strip\.is-chrome \{[\s\S]*?justify-content: flex-end;/);
    expect(notesSidebar).not.toMatch(/notes-sidebar-label/);
    expect(notesSidebar).not.toMatch(/>Notes</);
    expect(css).toMatch(/#sidebar \{[\s\S]*?--notes-rail: 16px;/);
    expect(css).toMatch(/\.topic-filter \{[\s\S]*?padding: 8px 10px 6px var\(--notes-rail\)/);
    expect(css).toMatch(/\.tag-filter \{[\s\S]*?padding: 8px 10px 6px var\(--notes-rail\)/);
    expect(css).toMatch(/\.date-tab \{[\s\S]*?padding: 8px 16px 8px var\(--notes-rail\)/);
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
