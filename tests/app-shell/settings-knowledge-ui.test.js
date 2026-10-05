import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { listFrontendSourceFiles, readHostApiSource, readShellHtml } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const appShellSrc = listFrontendSourceFiles(join(repoRoot, 'frontend/src/app-shell'))
  .sort()
  .map((abs) => readFileSync(abs, 'utf8'))
  .join('\n');
const indexHtml = [readShellHtml(), appShellSrc].join('\n');
const mainJs = appShellSrc;
const apiJs = readHostApiSource();
const settingsDialogJs = appShellSrc;

describe('Settings Knowledge UI', () => {
  it('Knowledge settings hold list, add, and category tabs', () => {
    expect(indexHtml).not.toContain('id="btn-repo-menu"');
    expect(indexHtml).not.toContain('id="repo-menu-wrap"');
    expect(indexHtml).toContain('id="settings-panel-knowledge"');
    expect(indexHtml).toContain('id="settings-tab-knowledge-list"');
    expect(indexHtml).toContain('id="settings-tab-knowledge-add"');
    expect(indexHtml).toContain('id="settings-tab-knowledge-categories"');
    expect(indexHtml).toContain('id="btn-sediment-kb-add-submit"');
    expect(indexHtml).toContain('id="btn-sediment-kb-manage-add"');
    expect(indexHtml).toContain('id="btn-repo-list-refresh"');
    expect(indexHtml).not.toContain('id="btn-repo-list"');
  });

  it('removes full knowledge sync button from list dialog', () => {
    expect(indexHtml).not.toContain('id="btn-kb-corpus-sync"');
  });

  it('add and manage live in Settings, not standalone dialogs', () => {
    expect(indexHtml).not.toContain('id="sediment-kb-add-dialog"');
    expect(indexHtml).not.toContain('id="sediment-kb-manage-dialog"');
    expect(indexHtml).not.toContain('id="repo-list-dialog"');
    expect(indexHtml).not.toContain('id="knowledge-root-dialog"');
    expect(indexHtml).toContain('id="settings-panel-knowledge"');
    expect(indexHtml).toContain('id="knowledge-root-path"');
    expect(indexHtml).toMatch(/id="knowledge-root-path"[^>]*readOnly/);
    expect(indexHtml).not.toContain('id="btn-knowledge-root-save"');
    expect(indexHtml).not.toContain('id="knowledge-root-error"');
    expect(indexHtml).toContain('id="settings-kb-hide-list"');
    expect(indexHtml).not.toContain('id="settings-kb-root"');
    expect(indexHtml).not.toContain('id="kb-setting-nav"');
    expect(indexHtml).not.toContain('id="settings-tab-spark-directory"');
    expect(indexHtml).not.toContain('id="settings-tab-spark-connection"');
    expect(indexHtml).toContain('id="settings-panel-notes"');
    expect(indexHtml).toContain('id="settings-tab-notes-directory"');
    expect(indexHtml).toContain('id="notes-root-path"');
    expect(indexHtml).toMatch(/id="notes-root-path"[^>]*readOnly/);
    expect(indexHtml).toContain('id="settings-tab-notes-add"');
    expect(indexHtml).toContain('id="settings-tab-notes-edit"');
    expect(indexHtml).toMatch(/data-tab="directory"[^>]*>\s*Directory\s*</);
    expect(indexHtml).toMatch(/data-tab="add"[^>]*>\s*Add category\s*</);
    expect(indexHtml).toMatch(/data-tab="edit"[^>]*>\s*Edit &amp; Delete category\s*</);
    const notesTabs = indexHtml.match(/id="settings-panel-notes"[\s\S]*?role="tablist">([\s\S]*?)<\/div>/)?.[1] ?? '';
    expect(notesTabs.indexOf('data-tab="directory"')).toBeGreaterThan(-1);
    expect(notesTabs.indexOf('data-tab="directory"')).toBeLessThan(
      notesTabs.indexOf('data-tab="add"'),
    );
    expect(notesTabs.indexOf('data-tab="add"')).toBeLessThan(
      notesTabs.indexOf('data-tab="edit"'),
    );
    expect(indexHtml).toContain('id="notes-cat-list"');
    expect(indexHtml).toContain('id="notes-cat-edit-dialog"');
    expect(indexHtml).toContain('id="notes-cat-edit-dialog-box"');
    expect(indexHtml).not.toContain('id="settings-tab-notes-categories"');
    expect(indexHtml).not.toContain('id="settings-tab-spark-categories"');
    expect(indexHtml).toContain('id="settings-tab-knowledge-list"');
    expect(indexHtml).toContain('id="settings-tab-knowledge-add"');
    expect(indexHtml).toContain('id="settings-tab-knowledge-categories"');
    expect(indexHtml).toContain('id="settings-tab-knowledge-directory"');
    expect(indexHtml).toContain('id="settings-tab-knowledge-hidden"');
    expect(indexHtml).toContain('id="settings-tab-llm-engine"');
    expect(indexHtml).not.toContain('id="settings-tab-github-account"');
    expect(indexHtml).not.toContain('id="spark-connect-item"');
    expect(indexHtml).not.toContain('id="spark-connect-url"');
    expect(indexHtml).toContain('id="sediment-kb-add-name"');
    expect(indexHtml).not.toContain('id="sediment-kb-add-url"');
    expect(indexHtml).not.toContain('id="sediment-kb-add-category"');
    expect(indexHtml).not.toContain('id="sediment-kb-add-description"');
  });

  it('Add submits a directory name only', () => {
    expect(mainJs).toMatch(
      /function prepareSedimentKbAddForm\(\)[\s\S]*?getElementById\('sediment-kb-add-name'\)/,
    );
    expect(mainJs).toMatch(
      /api\.addSedimentKbRepo\(name\)/,
    );
    expect(apiJs).toMatch(/export async function addSedimentKbRepo\(\s*name:/);
    expect(apiJs).toMatch(/writePost\('\/api\/sediment-kb\/repos\/add', \{ name \}\)/);
    expect(apiJs).not.toMatch(/body\.description = description/);
  });

  it('main.js wires Knowledge list helpers', () => {
    expect(mainJs).toMatch(/function prepareSedimentKbAddForm\(/);
    expect(mainJs).toMatch(/function prepareSedimentKbManage\(/);
    expect(mainJs).toMatch(/function prepareSedimentKbList\(/);
    expect(mainJs).not.toMatch(/function openSedimentKbCorpusDialog\(/);
    expect(mainJs).not.toMatch(/openSettingsDialog\(\{ panel: 'knowledge'/);
    expect(settingsDialogJs).not.toMatch(/btn-knowledge-root-save/);
    expect(settingsDialogJs).not.toMatch(
      /api\.setConfig\(\{ knowledge_root: knowledgeRoot \}\)/,
    );
    expect(settingsDialogJs).toMatch(/loadKbHidePatterns\(/);
    expect(mainJs).toMatch(/function renderSedimentKbListByCategory\(/);
    expect(mainJs).toMatch(/function onInlineCategoryChange\(/);
    expect(mainJs).toMatch(/function onDeleteSedimentKbRepo\(/);
    expect(mainJs).not.toMatch(/btn-kb-corpus-sync/);
    expect(mainJs).not.toMatch(/btn-kb-setting-connect-add/);
  });

  it('lists configured repos without Sync or Link; Delete stays', () => {
    expect(settingsDialogJs).toMatch(/function onDeleteSedimentKbRepo\(/);
    expect(settingsDialogJs).toMatch(/api\.removeSedimentKbRepo\(/);
    expect(settingsDialogJs).not.toMatch(/function onSyncSedimentKbRepo\(/);
    expect(settingsDialogJs).not.toMatch(/className="repo-sync-btn"/);
    expect(settingsDialogJs).not.toMatch(/>Cloned</);
    expect(settingsDialogJs).not.toMatch(/Not cloned/);
    expect(settingsDialogJs).not.toMatch(/Link ↗/);
    expect(settingsDialogJs).not.toMatch(/className="repo-list-item-link"/);
    expect(settingsDialogJs).not.toMatch(/✎/);
    expect(settingsDialogJs).not.toMatch(/View local changes/);
    expect(settingsDialogJs).not.toMatch(/repo-diff-badge/);
    expect(settingsDialogJs).not.toMatch(/fetchKbDiffStatus/);
    expect(settingsDialogJs).not.toMatch(/openKnowledgeDiffDialog/);
    expect(settingsDialogJs).toMatch(/className="sediment-kb-inline-category"/);
    expect(settingsDialogJs).toMatch(/function prepareSedimentKbManage\(/);
  });
});

describe('Settings Data panel', () => {
  it('does not render Data nav or spark directory/connection markup', () => {
    expect(indexHtml).not.toMatch(/data-panel="spark"/);
    expect(indexHtml).not.toContain('id="settings-panel-spark"');
    expect(indexHtml).not.toContain('id="settings-tab-spark-directory"');
    expect(indexHtml).not.toContain('id="settings-tab-spark-connection"');
    expect(indexHtml).not.toContain('id="settings-spark-root"');
    expect(indexHtml).not.toContain('id="spark-connect-item"');
    expect(indexHtml).not.toContain('id="spark-connect-url"');
    expect(settingsDialogJs).not.toMatch(/function renderSparkConnection\(/);
    expect(settingsDialogJs).not.toMatch(/function applySparkRootInference\(/);
    expect(settingsDialogJs).not.toMatch(/api\.setConfig\(\{ spark_github_repo_url/);
    const nav = indexHtml.match(/<nav id="settings-nav">([\s\S]*?)<\/nav>/)?.[1] ?? '';
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
});

describe('Settings Sync panel', () => {
  it('does not render Sync nav or GitHub bind markup', () => {
    expect(indexHtml).not.toMatch(/data-panel="github"/);
    expect(indexHtml).not.toContain('id="settings-panel-github"');
    expect(indexHtml).not.toContain('id="settings-tab-github-account"');
    expect(indexHtml).not.toContain('id="settings-github-user-url"');
    expect(indexHtml).not.toContain('id="settings-github-token"');
    expect(indexHtml).not.toContain('id="btn-settings-save-github"');
    expect(settingsDialogJs).not.toMatch(/btn-settings-save-github/);
    expect(settingsDialogJs).not.toMatch(/setGithubUserUrl\(/);
    expect(settingsDialogJs).not.toMatch(/savedSnapshot/);
  });
});
