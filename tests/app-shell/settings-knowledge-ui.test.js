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

describe('Settings workspace UI', () => {
  it('has no Knowledge panel: knowledge is a workspace, managed beside Notes', () => {
    expect(indexHtml).not.toContain('id="btn-repo-menu"');
    expect(indexHtml).not.toContain('id="settings-panel-knowledge"');
    expect(indexHtml).not.toContain('id="settings-tab-knowledge-list"');
    expect(indexHtml).not.toContain('id="settings-tab-knowledge-add"');
    expect(indexHtml).not.toContain('id="settings-tab-knowledge-categories"');
    expect(indexHtml).not.toContain('id="settings-tab-knowledge-hidden"');
    expect(indexHtml).not.toContain('id="knowledge-root-path"');
    expect(indexHtml).not.toContain('id="settings-kb-hide-list"');
    expect(indexHtml).not.toContain('id="sediment-kb-add-name"');
    expect(indexHtml).not.toContain('id="btn-sediment-kb-add-submit"');
    expect(indexHtml).not.toContain('id="btn-sediment-kb-manage-add"');
    expect(indexHtml).not.toMatch(/data-panel="knowledge"/);
  });

  it('Workspaces settings manage workspaces on one page, without sub-tabs', () => {
    expect(indexHtml).toContain('id="settings-panel-notes"');
    expect(indexHtml).toContain('id="workspaces-root-path"');
    expect(indexHtml).toMatch(/id="workspaces-root-path"[^>]*readOnly/);
    expect(indexHtml).not.toContain('id="notes-root-path"');
    const panel = readFileSync(join(repoRoot, 'frontend/src/app-shell/ui/settings/notes-panel.tsx'), 'utf8');
    expect(panel).not.toContain('role="tablist"');
    expect(panel).not.toContain('settings-tab-panel');
    expect(panel.indexOf('workspaces-root-path')).toBeLessThan(panel.indexOf('<NotesWorkspacesHost'));
    expect(indexHtml).not.toContain('Edit &amp; Delete project');
    expect(indexHtml).toContain('id="notes-workspace-list"');
    expect(indexHtml).toContain('Confirm disable');
    expect(indexHtml).toContain('Disabled (');
    // Project add/edit stay reachable until each workspace has its own page.
    expect(indexHtml).toContain('id="settings-tab-notes-add"');
    expect(indexHtml).toContain('id="settings-tab-notes-edit"');
    expect(indexHtml).toContain('id="notes-cat-list"');
    expect(indexHtml).toContain('id="notes-cat-edit-dialog"');
    expect(indexHtml).toContain('id="notes-cat-edit-dialog-box"');
    expect(indexHtml).toContain('id="settings-tab-llm-engine"');
  });

  it('no standalone add / manage / list dialogs and no kb settings code', () => {
    expect(indexHtml).not.toContain('id="sediment-kb-add-dialog"');
    expect(indexHtml).not.toContain('id="sediment-kb-manage-dialog"');
    expect(indexHtml).not.toContain('id="repo-list-dialog"');
    expect(indexHtml).not.toContain('id="knowledge-root-dialog"');
    expect(settingsDialogJs).not.toMatch(/loadKbHidePatterns/);
    expect(settingsDialogJs).not.toMatch(/SedimentKb/);
    expect(apiJs).not.toMatch(/sediment-kb/);
    expect(apiJs).not.toMatch(/\/api\/kb\//);
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
    expect(nav).not.toMatch(/data-panel="knowledge"/);
    expect(nav.indexOf('data-panel="notes"')).toBeLessThan(
      nav.indexOf('data-panel="llm"'),
    );
    expect(nav.indexOf('data-panel="llm"')).toBeLessThan(
      nav.indexOf('data-panel="mcp"'),
    );
    expect(nav).not.toMatch(/data-panel="github"/);
  });
});

describe('Settings field label spacing', () => {
  it('keeps a wider gap between a field label and its input', () => {
    const css = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');
    const label = css.slice(css.indexOf('.settings-field label {'));
    expect(label.slice(0, 180)).toMatch(/margin-bottom:\s*8px/);
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
