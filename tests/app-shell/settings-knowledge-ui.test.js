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
    expect(indexHtml).toContain('id="settings-kb-hide-list"');
    expect(indexHtml).not.toContain('id="settings-kb-root"');
    expect(indexHtml).not.toContain('id="kb-setting-nav"');
    expect(indexHtml).toContain('id="settings-tab-spark-directory"');
    expect(indexHtml).toContain('id="settings-tab-spark-connection"');
    expect(indexHtml).toContain('id="settings-panel-notes"');
    expect(indexHtml).toContain('id="settings-tab-notes-add"');
    expect(indexHtml).toContain('id="settings-tab-notes-edit"');
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
    expect(indexHtml).toContain('id="settings-tab-github-account"');
    expect(indexHtml).toContain('id="spark-connect-item"');
    expect(indexHtml).toMatch(/htmlFor="spark-connect-url">GitHub repository URL</);
    expect(indexHtml).toContain('id="sediment-kb-add-url"');
    expect(indexHtml).toContain('id="sediment-kb-add-category"');
    expect(indexHtml).toContain('id="sediment-kb-add-description"');
  });

  it('add dialog description field is optional textarea', () => {
    const match = indexHtml.match(/<textarea id="sediment-kb-add-description"[^>]*>/);
    expect(match).not.toBeNull();
  });

  it('prepareSedimentKbAddForm clears description input', () => {
    expect(mainJs).toMatch(
      /function prepareSedimentKbAddForm\(\)[\s\S]*?getElementById\('sediment-kb-add-description'\)\.value = ''/
    );
  });

  it('submit handler passes description to addSedimentKbRepo', () => {
    expect(mainJs).toMatch(
      /btn-sediment-kb-add-submit[\s\S]*?getElementById\('sediment-kb-add-description'\)[\s\S]*?api\.addSedimentKbRepo\([^)]*description/
    );
  });

  it('addSedimentKbRepo includes description in request body', () => {
    expect(apiJs).toMatch(
      /export async function addSedimentKbRepo\(\s*fullName\??(?::[^,)]+)?,\s*categoryId\??(?::[^,)]+)?,\s*description\??(?::[^,)]+)?/,
    );
    expect(apiJs).toMatch(/body\.description = description/);
  });

  it('main.js wires Knowledge list helpers', () => {
    expect(mainJs).toMatch(/function prepareSedimentKbAddForm\(/);
    expect(mainJs).toMatch(/function prepareSedimentKbManage\(/);
    expect(mainJs).toMatch(/function prepareSedimentKbList\(/);
    expect(mainJs).not.toMatch(/function openSedimentKbCorpusDialog\(/);
    expect(mainJs).not.toMatch(/openSettingsDialog\(\{ panel: 'knowledge'/);
    expect(settingsDialogJs).toMatch(
      /btn-knowledge-root-save[\s\S]*?api\.setConfig\(\{ knowledge_root: knowledgeRoot \}\)/
    );
    expect(settingsDialogJs).toMatch(/loadKbHidePatterns\(/);
    expect(mainJs).toMatch(/function renderSedimentKbListByCategory\(/);
    expect(mainJs).toMatch(/function onInlineCategoryChange\(/);
    expect(mainJs).toMatch(/function onDeleteSedimentKbRepo\(/);
    expect(mainJs).not.toMatch(/btn-kb-corpus-sync/);
    expect(mainJs).not.toMatch(/btn-kb-setting-connect-add/);
  });
});

describe('Spark GitHub connection', () => {
  it('persists a single spark_github_repo_url and can delete it', () => {
    expect(settingsDialogJs).toMatch(/function normalizeSparkGithubRepoUrl\(/);
    expect(settingsDialogJs).toMatch(/function renderSparkConnection\(/);
    expect(settingsDialogJs).toMatch(
      /api\.setConfig\(\{ spark_github_repo_url: repoUrl \}\)/,
    );
    expect(settingsDialogJs).toMatch(/async function deleteSparkGithubRepo\(/);
    expect(settingsDialogJs).toMatch(/btn-spark-connect-delete/);
    expect(settingsDialogJs).toMatch(/target="_blank"/);
  });

  it('locks the inferred origin repo and hides Delete', () => {
    expect(settingsDialogJs).toMatch(/function applySparkGithubRepoFromInferResponse\(/);
    expect(settingsDialogJs).toMatch(/resp\?\.spark_github_repo_url/);
    expect(settingsDialogJs).toMatch(/renderSparkConnection\(inferred, \{ locked: true \}\)/);
    expect(settingsDialogJs).toMatch(/Inferred from spark directory git origin \(read-only\)/);
  });

  it('re-infers on Directory blur even when the path matches the saved snapshot', () => {
    expect(settingsDialogJs).not.toMatch(
      /if \(!root \|\| root === savedSnapshot\.sparkRoot\)/,
    );
    expect(settingsDialogJs).toContain(
      'const pathChanged = root !== savedSnapshot.sparkRoot',
    );
  });

  it('does not disguise infer API errors as missing git origin', () => {
    expect(settingsDialogJs).toMatch(
      /return \{ ok: false, error: e\.message \|\| String\(e\) \}/,
    );
  });

  it('blocks Spark Connection binding until a Sync token is saved', () => {
    expect(settingsDialogJs).toMatch(/function isGithubAccountConfigured\(/);
    expect(settingsDialogJs).toMatch(/savedSnapshot\.hasGithubToken/);
    expect(settingsDialogJs).toMatch(
      /isGithubAccountConfigured[\s\S]*?savedSnapshot\.hasGithubToken[\s\S]*?savedSnapshot\.githubUserUrl/,
    );
    expect(settingsDialogJs).toMatch(/function syncSparkConnectionAccess\(/);
    expect(settingsDialogJs).toMatch(/Set a Sync token first to bind a data-store repository/);
    expect(indexHtml).toMatch(
      /<nav id="settings-nav">[\s\S]*data-panel="spark"[\s\S]*data-panel="notes"[\s\S]*data-panel="knowledge"[\s\S]*data-panel="llm"[\s\S]*data-panel="github">Sync/,
    );
    expect(indexHtml).not.toMatch(
      /data-panel="spark"[^>]*\bdisabled\b/,
    );
  });

  it('keeps GitHub profile above the Sync token', () => {
    const panelAt = indexHtml.indexOf('id="settings-panel-github"');
    const profileAt = indexHtml.indexOf('id="settings-github-user-url"');
    const tokenAt = indexHtml.indexOf('id="settings-github-token"');
    expect(panelAt).toBeGreaterThan(-1);
    expect(profileAt).toBeGreaterThan(panelAt);
    expect(tokenAt).toBeGreaterThan(profileAt);
  });
});
