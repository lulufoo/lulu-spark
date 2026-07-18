import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const indexHtml = readFileSync(join(repoRoot, 'frontend/index.html'), 'utf8');

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
    expect(extractTagOuter(indexHtml, 'btn-nav-home-title')).toContain('LuLu Workbench');
    expect(extractTagOuter(indexHtml, 'btn-repo-menu')).toContain('Knowledge');
    expect(extractTagOuter(indexHtml, 'btn-nav-home')).toContain('← Home');
    expect(indexHtml).not.toMatch(/id="btn-repo-menu"[^>]*>[^<]*沉淀知识库/);
  });

  it('header menus use table B labels', () => {
    expect(extractTagOuter(indexHtml, 'btn-sync-menu')).toContain('⇕ Sync');
    expect(extractTagOuter(indexHtml, 'btn-tools-menu')).toContain('⛓ Tools');
    expect(extractTagOuter(indexHtml, 'btn-settings')).toMatch(/Settings/);
    expect(extractTagOuter(indexHtml, 'btn-push-index')).toContain('↑ Commit changes');
    expect(extractTagOuter(indexHtml, 'btn-pull')).toContain('↓ Update project');
    expect(extractTagOuter(indexHtml, 'btn-local-refresh')).toContain('⟳ Refresh local');
  });

  it('search placeholders and rebuild titles use table B', () => {
    expect(indexHtml).toMatch(/id="gs-wb-input"[^>]*placeholder="Search notes…"/);
    expect(indexHtml).toMatch(/id="gs-kb-input"[^>]*placeholder="Search knowledge…"/);
    expect(indexHtml).toMatch(/id="gs-wb-rebuild-btn"[^>]*title="Rebuild Workbench index"/);
    expect(indexHtml).toMatch(/id="gs-kb-rebuild-btn"[^>]*title="Rebuild knowledge index"/);
  });

  it('repo list title uses table B2 knowledge list label', () => {
    expect(indexHtml).toMatch(/id="repo-list-title-group"[\s\S]*?☰ Knowledge list/);
    expect(indexHtml).not.toContain('☰ 沉淀知识库列表');
  });

  it('sidebar resizer aria-label uses table B', () => {
    expect(indexHtml).toMatch(/id="sidebar-resizer"[^>]*aria-label="Resize sidebar"/);
  });

  it('read-later dialog title uses table A brand name', () => {
    expect(indexHtml).toMatch(/<h3>Read Later<\/h3>/);
    expect(indexHtml).not.toContain('Read Later 待读');
  });

  it('settings nav uses table B2 directories and GitHub account labels', () => {
    expect(indexHtml).toMatch(/data-panel="directories">Directories</);
    expect(indexHtml).toMatch(/data-panel="github">GitHub account</);
    expect(indexHtml).toMatch(/data-panel="knowledge">Knowledge</);
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
