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
    expect(indexHtml).not.toContain('id="sediment-kb-add-name"');
    expect(indexHtml).not.toContain('id="btn-sediment-kb-add-submit"');
    expect(indexHtml).not.toContain('id="btn-sediment-kb-manage-add"');
    expect(indexHtml).not.toMatch(/data-panel="knowledge"/);
  });

  it('Workspaces settings use Tabs: Directory, Add workspace, Edit & Disable workspace', () => {
    const panel = readFileSync(join(repoRoot, 'frontend/src/app-shell/ui/settings/notes-panel.tsx'), 'utf8');
    expect(indexHtml).toContain('id="settings-panel-notes"');
    expect(panel).toContain('role="tablist"');
    for (const tab of ['directory', 'add', 'edit']) {
      expect(panel).toContain(`data-tab="${tab}"`);
      expect(panel).toContain(`id="settings-tab-notes-${tab}"`);
    }
    expect(panel).toContain('Add workspace');
    expect(panel).toContain('Edit &amp; Disable workspace');
    expect(indexHtml).toMatch(/id="workspaces-root-path"[^>]*readOnly/);
    expect(panel.indexOf('workspaces-root-path')).toBeLessThan(panel.indexOf('<NotesWorkspacesAddHost'));
    expect(indexHtml).not.toContain('id="notes-root-path"');
    expect(indexHtml).not.toContain('Edit &amp; Delete project');
    // Whole-row buttons open a dialog; Disable lives in the dialog behind a confirm step.
    expect(indexHtml).toContain('id="notes-ws-edit-dialog"');
    expect(indexHtml).toContain('id="notes-ws-edit-dialog-box"');
    expect(indexHtml).toContain('Disable workspace');
    expect(indexHtml).toContain('Disabled (');
  });

  it('a workspace page uses Tabs: Directory, Add project, Edit & Disable project, Hidden files', () => {
    const panel = readFileSync(
      join(repoRoot, 'frontend/src/app-shell/ui/settings/workspace-projects-panel.tsx'),
      'utf8',
    );
    expect(indexHtml).toContain('id="settings-panel-workspace"');
    expect(indexHtml).toContain('data-panel={`workspace:${workspace.workspace_id}`}');
    for (const tab of ['directory', 'add', 'edit']) {
      expect(panel).toContain(`data-tab="${tab}"`);
      expect(panel).toContain(`id="settings-tab-workspace-${tab}"`);
    }
    expect(panel).toContain('data-tab="hidden"');
    expect(panel).toContain('<HiddenFilesTab />');
    expect(indexHtml).toContain('id="settings-tab-workspace-hidden"');
    expect(indexHtml).toContain('id="settings-kb-hide-list"');
    expect(indexHtml).toContain('id="btn-settings-kb-hide-add"');
    expect(indexHtml).toContain('id="workspace-dir-path"');
    expect(indexHtml).toContain('id="notes-cat-list"');
    expect(indexHtml).toContain('id="notes-cat-edit-dialog"');
    expect(indexHtml).toContain('id="notes-cat-edit-dialog-box"');
    expect(indexHtml).toContain('id="settings-tab-llm-engine"');
    const nav = readFileSync(join(repoRoot, 'frontend/src/app-shell/ui/settings/workspace-nav.tsx'), 'utf8');
    expect(nav).not.toContain('settings-nav-workspace');
    expect(readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8')).not.toContain('.settings-nav-workspace');
  });

  it('no standalone add / manage / list dialogs and no kb settings code', () => {
    expect(indexHtml).not.toContain('id="sediment-kb-add-dialog"');
    expect(indexHtml).not.toContain('id="sediment-kb-manage-dialog"');
    expect(indexHtml).not.toContain('id="repo-list-dialog"');
    expect(indexHtml).not.toContain('id="knowledge-root-dialog"');
    expect(settingsDialogJs).not.toMatch(/loadKbHidePatterns/);
    expect(settingsDialogJs).not.toMatch(/fetchKbHidePatterns/);
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
      nav.indexOf('data-panel="document"'),
    );
    expect(nav.indexOf('data-panel="document"')).toBeLessThan(
      nav.indexOf('data-panel="llm"'),
    );
    expect(nav.indexOf('data-panel="llm"')).toBeLessThan(
      nav.indexOf('data-panel="mcp"'),
    );
    expect(nav.indexOf('data-panel="mcp"')).toBeLessThan(
      nav.indexOf('data-panel="devices"'),
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
