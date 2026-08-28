import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { readShellHtml } from '../helpers/read-frontend-js.js';
import { readHostApiSource, readMainSource, readSettingsDialogSource } from '../helpers/read-frontend-js.js';

const indexHtml = readShellHtml();
const mainJs = readMainSource();
const apiJs = readHostApiSource();
const settingsDialogJs = readSettingsDialogSource();

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

  it('removes full corpus sync button from list dialog', () => {
    expect(indexHtml).not.toContain('id="btn-kb-corpus-sync"');
  });

  it('add and manage live in Settings, not standalone dialogs', () => {
    expect(indexHtml).not.toContain('id="sediment-kb-add-dialog"');
    expect(indexHtml).not.toContain('id="sediment-kb-manage-dialog"');
    expect(indexHtml).not.toContain('id="repo-list-dialog"');
    expect(indexHtml).not.toContain('id="sediment-kb-corpus-dialog"');
    expect(indexHtml).toContain('id="settings-panel-knowledge"');
    expect(indexHtml).toContain('id="sediment-kb-corpus-path"');
    expect(indexHtml).toContain('id="settings-kb-hide-pattern"');
    expect(indexHtml).not.toContain('id="settings-kb-root"');
    expect(indexHtml).not.toContain('id="kb-setting-nav"');
    expect(indexHtml).toContain('id="settings-tab-notes-directory"');
    expect(indexHtml).toContain('id="settings-tab-notes-connection"');
    expect(indexHtml).toContain('id="settings-tab-knowledge-list"');
    expect(indexHtml).toContain('id="settings-tab-knowledge-add"');
    expect(indexHtml).toContain('id="settings-tab-knowledge-categories"');
    expect(indexHtml).toContain('id="settings-tab-knowledge-directory"');
    expect(indexHtml).toContain('id="settings-tab-knowledge-hidden"');
    expect(indexHtml).toContain('id="settings-tab-llm-engine"');
    expect(indexHtml).toContain('id="settings-tab-github-account"');
    expect(indexHtml).toContain('id="notes-connect-item"');
    expect(indexHtml).toMatch(/htmlFor="notes-connect-url">GitHub repository URL</);
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
    expect(apiJs).toMatch(/export async function addSedimentKbRepo\(fullName, categoryId, description\)/);
    expect(apiJs).toMatch(/body\.description = description/);
  });

  it('main.js wires Knowledge list helpers', () => {
    expect(mainJs).toMatch(/function prepareSedimentKbAddForm\(/);
    expect(mainJs).toMatch(/function prepareSedimentKbManage\(/);
    expect(mainJs).toMatch(/function prepareSedimentKbList\(/);
    expect(mainJs).not.toMatch(/function openSedimentKbCorpusDialog\(/);
    expect(mainJs).not.toMatch(/openSettingsDialog\(\{ panel: 'knowledge'/);
    expect(settingsDialogJs).toMatch(
      /btn-sediment-kb-corpus-save[\s\S]*?api\.setConfig\(\{ knowledge_corpus_root: knowledgeCorpusRoot \}\)/
    );
    expect(settingsDialogJs).toMatch(/btn-settings-save-knowledge[\s\S]*?saveKbHidePattern\(pattern\)/);
    expect(mainJs).toMatch(/function renderSedimentKbListByCategory\(/);
    expect(mainJs).toMatch(/function onInlineCategoryChange\(/);
    expect(mainJs).toMatch(/function onDeleteSedimentKbRepo\(/);
    expect(mainJs).not.toMatch(/btn-kb-corpus-sync/);
    expect(mainJs).not.toMatch(/btn-kb-setting-connect-add/);
  });
});

describe('Notes GitHub connection', () => {
  it('persists a single workbench_github_repo_url and can delete it', () => {
    expect(settingsDialogJs).toMatch(/function normalizeNotesGithubRepoUrl\(/);
    expect(settingsDialogJs).toMatch(/function renderNotesConnection\(/);
    expect(settingsDialogJs).toMatch(
      /api\.setConfig\(\{ workbench_github_repo_url: repoUrl \}\)/,
    );
    expect(settingsDialogJs).toMatch(/async function deleteNotesGithubRepo\(/);
    expect(settingsDialogJs).toMatch(/btn-notes-connect-delete/);
    expect(settingsDialogJs).toMatch(/target="_blank"/);
  });

  it('locks the inferred origin repo and hides Delete', () => {
    expect(settingsDialogJs).toMatch(/function applyNotesGithubRepoFromInferResponse\(/);
    expect(settingsDialogJs).toMatch(/resp\?\.workbench_github_repo_url/);
    expect(settingsDialogJs).toMatch(/renderNotesConnection\(inferred, \{ locked: true \}\)/);
    expect(settingsDialogJs).toMatch(/Inferred from workbench directory git origin \(read-only\)/);
  });

  it('re-infers on Directory blur even when the path matches the saved snapshot', () => {
    expect(settingsDialogJs).not.toMatch(
      /if \(!root \|\| root === savedSnapshot\.workbenchKnowledgeRoot\)/,
    );
    expect(settingsDialogJs).toContain(
      'const pathChanged = root !== savedSnapshot.workbenchKnowledgeRoot',
    );
  });

  it('does not disguise infer API errors as missing git origin', () => {
    expect(settingsDialogJs).toMatch(
      /return \{ ok: false, error: e\.message \|\| String\(e\) \}/,
    );
  });

  it('blocks Notes Connection binding until a Sync token is saved', () => {
    expect(settingsDialogJs).toMatch(/function isGithubAccountConfigured\(/);
    expect(settingsDialogJs).toMatch(/savedSnapshot\.hasGithubToken/);
    expect(settingsDialogJs).toMatch(
      /isGithubAccountConfigured[\s\S]*?savedSnapshot\.hasGithubToken[\s\S]*?savedSnapshot\.githubUserUrl/,
    );
    expect(settingsDialogJs).toMatch(/function syncNotesConnectionAccess\(/);
    expect(settingsDialogJs).toMatch(/Set a Sync token first to bind a Notes repository/);
    expect(indexHtml).toMatch(
      /<nav id="settings-nav">[\s\S]*data-panel="directories"[\s\S]*data-panel="knowledge"[\s\S]*data-panel="llm"[\s\S]*data-panel="github">Sync/,
    );
    expect(indexHtml).not.toMatch(
      /data-panel="directories"[^>]*\bdisabled\b/,
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
