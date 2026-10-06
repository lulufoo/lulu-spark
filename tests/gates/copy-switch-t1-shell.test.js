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
    expect(css).toMatch(/\.home-chat-nav \{[\s\S]*?padding: 8px var\(--home-chat-row-inset\) 12px;/);
    expect(css).toMatch(/\.home-chat-nav-label \{[\s\S]*?padding: 6px 8px 8px;/);
    expect(css).toMatch(/\.home-chat-nav-item \{[\s\S]*?padding: 7px 8px;/);
    expect(css).toMatch(
      /\.home-chat-sessions-head \{[\s\S]*?padding: 14px var\(--home-chat-row-inset\) 8px var\(--home-rail\);/,
    );
    expect(css).toMatch(/\.home-chat-sessions \{[\s\S]*?padding: 2px var\(--home-chat-row-inset\) 16px;/);
    const llmMark = readFileSync(join(repoRoot, 'frontend/src/shared/llm-mark.tsx'), 'utf8');
    expect(llmMark).toMatch(/translate\(2 2\) scale\(1\.17647\) translate\(-3\.5 -3\.5\)/);
    expect(css).toMatch(/\.home-chat-session\.is-active \{\s*background: #fff;\s*\}/);
    expect(css).toMatch(/\.home-chat-session-open \{[\s\S]*?padding: 8px;/);
    expect(css).toMatch(/\.home-chat-session-icon \{[\s\S]*?width: 24px;/);
    expect(css).toMatch(/\.home-chat-nav \.home-desktop-shortcut-icon \{[\s\S]*?width: 18px;/);
    expect(css).not.toMatch(/\.home-chat-nav \.home-desktop-shortcut-icon \{[^}]*border:/);
    expect(css).not.toMatch(/\.home-chat-nav \.home-desktop-shortcut-icon \{[^}]*background:/);
    expect(css).not.toMatch(/font-variant-emoji/);
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
    expect(notesPage).toMatch(/id="md-header" className="viewer-header" data-tauri-drag-region="deep"/);
    expect(notesPage).not.toMatch(/KnowledgeSearchHost/);
    expect(css).not.toMatch(/#knowledge-panel/);
    expect(css).toMatch(/\.notes-page-toolbar,[\s\S]*?#md-header\.viewer-header,[\s\S]*?-webkit-app-region: drag;/);
    expect(css).toMatch(/#md-header \.viewer-header-actions,[\s\S]*?#md-header \.overlay-dismiss-button,[\s\S]*?-webkit-app-region: no-drag;/);
    expect(notesPage).toMatch(/hidden=\{showOutlet\}/);
    expect(notesPage).not.toMatch(/<NotesStatus \/>\s*<NotesDateHeading \/>/);
    expect(css).toMatch(/\.page-back-home \{[\s\S]*?border: none;/);
    expect(css).toMatch(/\.page-back-home-icon \{[\s\S]*?width: 14px;/);
    expect(css).toMatch(/\.page-back-home-icon \{[\s\S]*?color: #57606a;/);
    expect(css).not.toMatch(/page-back-home-chevron/);
    expect(css).not.toMatch(/\.page-back-home-icon \{[^}]*#0071e3/);
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
    expect(knowledgeSidebar).toMatch(/M20 20a2 2 0 0 0 2-2V8/);
    expect(knowledgeSidebar).toMatch(/M15 2H6a2 2 0 0 0-2 2v16/);
    expect(knowledgeSidebar).toMatch(/fill: 'none'/);
    expect(css).toMatch(/\.knowledge-doc-tree-icon \{[\s\S]*?color: #57606a;/);
    expect(css).not.toMatch(/\.knowledge-doc-tree-icon \{[^}]*#54aeff/);
    expect(css).toMatch(/\.knowledge-doc-tree-label\.selected \.knowledge-doc-tree-icon \{[\s\S]*?color: #57606a;/);
    expect(css).toMatch(/\.kb-reader-header\.viewer-header \{[\s\S]*?padding-left: 24px;/);
    expect(css).toMatch(/\.kb-reader-header\.viewer-header \{[\s\S]*?overflow: visible;/);
    expect(css).toMatch(/\.kb-reader \{[\s\S]*?overflow: visible;/);
    expect(css).toMatch(/\.knowledge-doc-reader-pane \{[\s\S]*?overflow: visible;/);
    expect(css).toMatch(/\.viewer-header-actions \{[\s\S]*?gap: 0;/);
    expect(css).toMatch(/#md-lang-bar \{[\s\S]*?margin-right: 10px;/);
    expect(css).toMatch(/#bind-dialog-box \{[\s\S]*?width: min\(720px, 94vw\);/);
    expect(css).toMatch(/#bind-dialog-box \{[\s\S]*?height: min\(560px, 86vh\);/);
    expect(css).toMatch(/#bind-dialog-box \{[\s\S]*?max-height: 86vh;/);
    expect(css).toMatch(/#read-later-dialog-box \{[\s\S]*?width: min\(720px, 94vw\);/);
    expect(css).toMatch(/#read-later-dialog-box \{[\s\S]*?height: min\(560px, 86vh\);/);
    expect(css).toMatch(/#read-later-dialog-box \{[\s\S]*?max-height: 86vh;/);
    expect(css).toMatch(/\.settings-select-menu \{[\s\S]*?z-index: 2100;/);
    const settingsDialog = readFileSync(join(repoRoot, 'frontend/src/app-shell/commands/settings/dialog.ts'), 'utf8');
    expect(settingsDialog).toMatch(/mountSettingsListSelects\(\)/);
    expect(css).toMatch(/\.comment-item \{[\s\S]*?background: #f6f8fa;/);
    expect(css).toMatch(/\.comment-float-btn \{[\s\S]*?background: #f6f8fa;/);
    expect(css).toMatch(/\.comment-editor-box \{[\s\S]*?border: 1px solid #eaeef2;/);
    expect(css).not.toMatch(/#fefef7/);
    const notesComments = readFileSync(join(repoRoot, 'frontend/src/notes/ui/comments.tsx'), 'utf8');
    expect(notesComments).not.toMatch(/💬/);
    expect(css).toMatch(/\.viewer-panel-title \{[\s\S]*?border-right: 1px solid #d8dee4;/);
    expect(css).toMatch(/\.viewer-panel-title \{[\s\S]*?font-size: 14px;/);
    expect(css).not.toMatch(/\.kb-reader-header \.viewer-header-rule/);
    expect(css).toMatch(/\.kb-reader-header \.gs-search-wrap \{[\s\S]*?margin-left: auto;/);
    expect(css).toMatch(/\.kb-reader-header \.gs-search-dropdown \{[\s\S]*?right: 0;/);
    expect(css).toMatch(/\.kb-reader-header\.viewer-header \{[\s\S]*?-webkit-app-region: drag;/);
    expect(css).toMatch(
      /\.kb-reader-header-actions,[\s\S]*?\.kb-reader-header \.gs-search-dropdown \{[\s\S]*?-webkit-app-region: no-drag;/,
    );
    const knowledgeShell = readFileSync(join(repoRoot, 'frontend/src/knowledge/ui/viewer/shell.tsx'), 'utf8');
    expect(knowledgeShell).toMatch(/class="kb-reader-header viewer-header" data-tauri-drag-region="deep"/);
    expect(knowledgeShell).toMatch(/className="kb-reader-header viewer-header" data-tauri-drag-region="deep"/);
    expect(knowledgeShell).toMatch(/<KnowledgeSearch hidden=\{false\} \/>/);
    expect(knowledgeShell).not.toMatch(/viewer-header-rule/);
    expect(knowledgeShell).toMatch(/kb-reader-header-actions[\s\S]*<KnowledgeSearch hidden=\{false\} \/>/);
    expect(knowledgeShell).toMatch(/className="viewer-header-actions kb-reader-header-actions"/);
    expect(knowledgeShell).toMatch(/kb-btn-add-comment/);
    expect(notesPage).toMatch(/className="viewer-header-actions"/);
    expect(notesPage).toMatch(/id="btn-add-comment"/);
    expect(notesPage).toMatch(/<ViewerHeaderIcon name="copy" \/>/);
    expect(notesPage).toMatch(/<ViewerHeaderIcon name="chat" filled \/>/);
    expect(notesPage).toMatch(/<ViewerHeaderIcon name="edit" \/>/);
    expect(notesPage).toMatch(/<ViewerHeaderIcon name="comment" \/>/);
    expect(notesPage).not.toMatch(/<ViewerHeaderIcon name="copy" \/> Copy/);
    expect(notesPage).toMatch(/id="btn-cancel-edit"[\s\S]*?id="btn-open-in-chat"[\s\S]*?id="md-close"/);
    expect(notesPage).toMatch(/id="btn-save"[\s\S]*?\{viewer\.saving \? 'Saving…' : 'Save'\}/);
    expect(notesPage).not.toMatch(/id="btn-save"[\s\S]*?primary/);
    expect(notesPage).not.toMatch(/💾 Save/);
    expect(knowledgeShell).not.toMatch(/kb-btn-save[\s\S]*?primary/);
    expect(knowledgeShell).not.toMatch(/💾 Save/);
    expect(knowledgeShell).toMatch(/<ViewerHeaderIcon name="copy" \/>/);
    expect(knowledgeShell).toMatch(/<ViewerHeaderIcon name="chat" filled \/>/);
    expect(knowledgeShell).toMatch(/<ViewerHeaderIcon name="edit" \/>/);
    expect(knowledgeShell).toMatch(/<ViewerHeaderIcon name="comment" \/>/);
    expect(knowledgeShell).not.toMatch(/<ViewerHeaderIcon name="copy" \/> Copy/);
    expect(knowledgeShell).toMatch(/kb-btn-cancel-edit[\s\S]*?kb-btn-open-in-chat/);
    expect(notesPage).not.toMatch(/📁|🗨️|✏️|💬/);
    expect(css).toMatch(/\.viewer-header-actions \.md-header-btn \{[\s\S]*?padding: 5px 10px;/);
    expect(css).toMatch(/\.viewer-header-actions \.md-header-btn \{[\s\S]*?gap: 6px;/);
    expect(css).toMatch(/\.viewer-header-actions \.viewer-header-icon \{[\s\S]*?width: 14px;/);
    expect(css).toMatch(/\.viewer-header-actions \.viewer-header-icon\.is-filled \{[\s\S]*?fill: currentColor;/);
    const headerIcons = readFileSync(join(repoRoot, 'frontend/src/shared/viewer-header-icons.tsx'), 'utf8');
    expect(headerIcons).toMatch(/copy:/);
    expect(headerIcons).toMatch(/chat:/);
    expect(headerIcons).toMatch(/edit:/);
    expect(headerIcons).toMatch(/comment:/);
    expect(headerIcons).toMatch(/M7\.9 20A9 9 0 1 0 4 16\.1L2 22Z/);
    expect(headerIcons).toMatch(/M21\.174 6\.812/);
    expect(headerIcons).toMatch(/M21 15a2 2 0 0 1-2 2H7l-4 4V5/);
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
    expect(homeMark).toMatch(/page-back-home-icon/);
    expect(homeMark).toMatch(/m12 19-7-7 7-7/);
    expect(homeMark).toMatch(/M19 12H5/);
    expect(homeMark).not.toMatch(/←/);
    expect(css).toMatch(/\.window-drag-strip\.is-chrome \{[\s\S]*?justify-content: flex-end;/);
    expect(css).not.toMatch(/window-fullscreen-brand/);
    const dragStrip = readFileSync(join(repoRoot, 'frontend/src/shared/window-drag-strip.tsx'), 'utf8');
    expect(dragStrip).not.toMatch(/Lulu Spark/);
    expect(dragStrip).not.toMatch(/isFullscreen/);
    expect(css).toMatch(/#sidebar \.window-drag-strip\.is-chrome \{[\s\S]*?padding-right: var\(--notes-rail\);/);
    expect(css).toMatch(/\.knowledge-doc-sidebar \.window-drag-strip\.is-chrome \{[\s\S]*?padding-right: 16px;/);
    expect(css).toMatch(/#sidebar \.window-drag-strip \.page-back-home,[\s\S]*?padding-right: 0;/);
    expect(notesSidebar).not.toMatch(/notes-sidebar-label/);
    expect(notesSidebar).not.toMatch(/>Notes</);
    expect(css).toMatch(/#sidebar \{[\s\S]*?--notes-rail: 16px;/);
    expect(css).toMatch(/\.topic-filter \{[\s\S]*?padding: 8px var\(--notes-rail\) 6px;/);
    expect(css).toMatch(/\.topic-count \{[\s\S]*?text-align: right;/);
    expect(css).toMatch(/\.tag-count \{[\s\S]*?text-align: right;/);
    expect(notesSidebar).not.toMatch(/\/ \$\{total\} items/);
    expect(css).toMatch(/\.badge-done \{[\s\S]*?border-color: transparent;/);
    expect(css).toMatch(/\.doc-card\.done \.badge-done \{[\s\S]*?border-color: transparent;/);
    expect(css).not.toMatch(/\.badge-done \{[^}]*border-color: #e2e8ee/);
    expect(css).not.toMatch(/\.topic-filter \{[^}]*border-bottom:/);
    expect(css).toMatch(/\.tag-filter \{[\s\S]*?padding: 8px var\(--notes-rail\) 16px;/);
    expect(css).toMatch(/\.tag-filter \{[\s\S]*?border-bottom: 1px solid #eaeef2;/);
    expect(css).toMatch(/\.knowledge-doc-sidebar-header \{[\s\S]*?padding: 8px 16px 16px;/);
    expect(css).toMatch(/\.knowledge-doc-sidebar-header \{[\s\S]*?border-bottom: 1px solid #eaeef2;/);
    expect(css).toMatch(/\.list-select-trigger \{[\s\S]*?border: 1px solid #eaeef2;/);
    expect(css).toMatch(/\.list-select-trigger \{[\s\S]*?background: #f6f8fa;/);
    expect(css).not.toMatch(/\.list-select-trigger \{[^}]*border: 1px solid #d0d7de/);
    expect(css).toMatch(/\.list-select-menu \{[\s\S]*?background: #fff;/);
    expect(css).toMatch(/\.list-select-menu \{[\s\S]*?border: 1px solid #eaeef2;/);
    expect(css).toMatch(/\.list-select-option\.selected \{[\s\S]*?background: #eef1f4;/);
    expect(css).not.toMatch(/\.list-select-option\.selected::before/);
    expect(css).not.toMatch(/\.list-select-chevron::before/);
    const listSelect = readFileSync(join(repoRoot, 'frontend/src/shared/floating-list-select.ts'), 'utf8');
    expect(listSelect).toMatch(/list-select-chevron-icon/);
    expect(listSelect).toMatch(/m6 9 6 6 6-6/);
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
    expect(homeSrc).toMatch(/<HomeNavIcon name="notes"/);
    expect(homeSrc).toMatch(/<HomeNavIcon name="knowledge"/);
    expect(homeSrc).toMatch(/<HomeNavIcon name="read-later"/);
    expect(homeSrc).toMatch(/<HomeNavIcon name="bind"/);
    expect(homeSrc).toMatch(/<HomeNavIcon name="settings"/);
    expect(homeSrc).not.toMatch(/📂|📚|📑|📱|📲|⚙️|⚙/);
    const navIcons = readFileSync(join(repoRoot, 'frontend/src/home/ui/nav-icons.tsx'), 'utf8');
    expect(navIcons).toMatch(/data-home-icon=\{name\}/);
    expect(navIcons).toMatch(/notes: \(/);
    expect(navIcons).toMatch(/M2 6h4/);
    expect(navIcons).toMatch(/knowledge: \(/);
    expect(navIcons).toMatch(/M12 7v14/);
    expect(navIcons).toMatch(/'read-later':/);
    expect(navIcons).toMatch(/m19 21-7-4-7 4V5/);
    expect(navIcons).toMatch(/bind: \(/);
    expect(navIcons).toMatch(/width="14" height="20" x="5" y="2"/);
    expect(navIcons).toMatch(/settings: \(/);
    expect(navIcons).toMatch(/<circle cx="12" cy="12" r="3"/);
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
