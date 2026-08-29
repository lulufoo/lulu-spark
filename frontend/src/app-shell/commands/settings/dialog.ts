import * as api from '../../../host/api.ts';
import { setGithubUserUrl } from '../../../host/constants.ts';
import { saveKbHidePattern } from '../../../knowledge/state/hide-pattern.ts';
import { state } from '../../../host/state.ts';
import { applyEngineCategorySelection, saveAssistantEnginePanel } from './engine.ts';
import {
  applyWorkbenchRootInference,
  clearGithubUserUrlInferredLock,
  syncGithubUserUrlLockFromWorkbenchRoot,
} from './github-user.ts';
import {
  clearMcpServerBlock,
  generateCursorIdeServerBlock,
  rotateCursorIdeTicket,
  revokeMcpSlotTicket,
} from './mcp.ts';
import {
  addNotesGithubRepo,
  isGithubAccountConfigured,
} from './notes-github.ts';
import {
  isGithubUserUrlInferredLocked,
  savedSnapshot,
  setResult,
  store,
} from '../../state/settings/store.ts';
import { settingsOpenStore } from '../../state/dialog-open.ts';
import { loadSettingsSnapshot } from './snapshot.ts';
import { switchPanel, switchSettingsTab } from '../../ui/settings/tabs.ts';

export { settingsOpenStore };

type SettingsOpenOpts = { panel?: string; tab?: string };
type Inference = {
  ok?: boolean;
  conflict?: boolean;
  existing?: string;
  inferred?: string;
  error?: string;
  locked?: boolean;
  noRemote?: boolean;
};

function input(id: string): HTMLInputElement {
  return document.getElementById(id) as HTMLInputElement;
}

function btn(id: string): HTMLButtonElement {
  return document.getElementById(id) as HTMLButtonElement;
}

let wired = false;

export function ensureWired() {
  if (wired) return;
  // renderToHtml paints into a detached box. Wiring against document there
  // throws and React drops the Settings subtree from the HTML snapshot.
  if (!document.getElementById('settings-dialog-box')) return;
  wireSettingsDialog();
  wired = true;
}

export function closeSettingsDialog() {
  settingsOpenStore.set(false);
  document.getElementById('settings-dialog')?.classList.remove('open');
}

export async function openSettingsDialog(opts: SettingsOpenOpts = {}) {
  settingsOpenStore.set(true);
  document.getElementById('settings-dialog')?.classList.add('open');
  ensureWired();
  setResult('settings-result-directories', '');
  setResult('notes-connect-error', '');
  setResult('knowledge-root-error', '');
  setResult('settings-result-knowledge', '');
  setResult('settings-result-github', '');
  setResult('settings-result-llm', '');
  setResult('settings-result-mcp', '');
  clearMcpServerBlock();
  const tokenInput = input('settings-github-token');
  const keyInput = input('settings-llm-api-key');
  if (tokenInput) tokenInput.value = '';
  if (keyInput) keyInput.value = '';
  const panelId = opts.panel || 'directories';
  switchSettingsTab('directories', panelId === 'directories' ? (opts.tab || 'directory') : 'directory');
  switchSettingsTab('knowledge', panelId === 'knowledge' ? (opts.tab || 'directory') : 'directory');
  switchSettingsTab('llm', 'engine');
  switchSettingsTab('github', 'account');
  switchPanel(panelId);
  await loadSettingsSnapshot();
}

function wireSettingsDialog() {
  document.querySelectorAll('.settings-tab').forEach((tabBtn) => {
    tabBtn.addEventListener('click', () => {
      const panel = tabBtn.closest('.settings-panel');
      const panelId = panel?.id?.replace(/^settings-panel-/, '');
      const tab = (tabBtn as HTMLElement).dataset.tab;
      if (panelId && tab) switchSettingsTab(panelId, tab);
    });
  });

  document.querySelectorAll('.settings-nav-item').forEach((navBtn) => {
    navBtn.addEventListener('click', async () => {
      const panelId = (navBtn as HTMLElement).dataset.panel;
      if (!panelId) return;
      switchPanel(panelId);
      if (panelId === 'github') {
        await syncGithubUserUrlLockFromWorkbenchRoot();
      }
    });
  });

  btn('btn-settings-close')?.addEventListener('click', closeSettingsDialog);
  btn('btn-settings-cancel')?.addEventListener('click', closeSettingsDialog);
  document.getElementById('settings-dialog')?.addEventListener('click', (e) => {
    if (e.target === document.getElementById('settings-dialog')) closeSettingsDialog();
  });
  document.getElementById('btn-notes-connect-add')?.addEventListener('click', () => {
    void addNotesGithubRepo();
  });
  document.getElementById('notes-connect-url')?.addEventListener('keydown', (e) => {
    if ((e as KeyboardEvent).key === 'Enter') document.getElementById('btn-notes-connect-add')?.click();
  });
  document.getElementById('btn-knowledge-root-save')?.addEventListener('click', () => {
    void (async () => {
      const pathInput = document.getElementById('knowledge-root-path') as HTMLInputElement | null;
      const saveBtn = document.getElementById('btn-knowledge-root-save') as HTMLButtonElement | null;
      const knowledgeRoot = pathInput?.value.trim() || '';
      if (!knowledgeRoot) {
        setResult('knowledge-root-error', 'Enter a directory path.', true);
        return;
      }
      if (saveBtn) saveBtn.disabled = true;
      setResult('knowledge-root-error', '');
      try {
        const resp = await api.setConfig({ knowledge_root: knowledgeRoot });
        if (resp?.error) throw new Error(resp.error);
        state.ui.knowledgeRoot = knowledgeRoot;
        setResult('knowledge-root-error', 'Saved: Knowledge directory.');
      } catch (err) {
        const e = err as Error;
        setResult('knowledge-root-error', e.message || String(e), true);
      } finally {
        if (saveBtn) saveBtn.disabled = false;
      }
    })();
  });
  document.getElementById('knowledge-root-path')?.addEventListener('keydown', (e) => {
    if ((e as KeyboardEvent).key === 'Enter') document.getElementById('btn-knowledge-root-save')?.click();
  });
  document.getElementById('btn-settings-save-knowledge')?.addEventListener('click', () => {
    const saveBtn = document.getElementById('btn-settings-save-knowledge') as HTMLButtonElement | null;
    const pattern = (document.getElementById('settings-kb-hide-pattern') as HTMLInputElement | null)?.value ?? '';
    if (saveBtn) saveBtn.disabled = true;
    setResult('settings-result-knowledge', '');
    try {
      const result = saveKbHidePattern(pattern);
      if (!result.ok) {
        setResult('settings-result-knowledge', `Invalid regex: ${result.error}`, true);
        return;
      }
      setResult('settings-result-knowledge', 'Hide rules saved.');
    } finally {
      if (saveBtn) saveBtn.disabled = false;
    }
  });
  document.getElementById('settings-kb-hide-pattern')?.addEventListener('keydown', (e) => {
    if ((e as KeyboardEvent).key === 'Enter') document.getElementById('btn-settings-save-knowledge')?.click();
  });

  input('settings-archive-root')?.addEventListener('input', () => {
    const root = input('settings-archive-root').value.trim();
    if (!root) {
      clearGithubUserUrlInferredLock();
    }
  });

  input('settings-archive-root')?.addEventListener('blur', async () => {
    const root = input('settings-archive-root').value.trim();
    if (!root) {
      return;
    }
    const pathChanged = root !== savedSnapshot.workbenchRoot;
    const inference = (await applyWorkbenchRootInference({ revertOnConflict: pathChanged })) as Inference;
    if (!inference.ok && inference.conflict) {
      setResult(
        'settings-result-directories',
        `Directory not saved: GitHub profile on Sync (${inference.existing}) does not match origin inference (${inference.inferred}). Fix or clear the profile on Sync before changing the directory.`,
        true,
      );
      return;
    }
    if (inference.error) {
      setResult(
        'settings-result-directories',
        `Could not infer GitHub profile: ${inference.error}. If you just updated the app, fully restart and try again.`,
        true,
      );
      return;
    }
    if (inference.locked && inference.inferred) {
      setResult(
        'settings-result-directories',
        `GitHub profile inferred from git origin (locked — save on Sync).`,
        false,
      );
      setResult('settings-result-github', '');
      if (pathChanged) switchPanel('directories');
      return;
    }
    if (inference.noRemote) {
      setResult(
        'settings-result-directories',
        'No git origin detected; could not auto-infer GitHub profile — enter it manually on Sync.',
        false,
      );
    }
  });

  btn('btn-settings-save-directories')?.addEventListener('click', async () => {
    const saveBtn = btn('btn-settings-save-directories');
    const archiveInput = input('settings-archive-root');
    const workbenchRoot = archiveInput.value.trim();
    const githubUserInput = input('settings-github-user-url');

    if (!workbenchRoot) {
      setResult('settings-result-directories', 'Enter a directory path.', true);
      return;
    }

    const includeWorkbenchRoot = true;
    let includeGithubUrl = false;
    const messages: string[] = [];

    try {
      const check = await api.checkWorkbenchRoot(workbenchRoot);
      if (check?.ok === false) {
        setResult(
          'settings-result-directories',
          check.error || 'Workbench directory invalid; not saved.',
          true,
        );
        return;
      }
    } catch (err) {
      const e = err as Error;
      setResult(
        'settings-result-directories',
        `Workbench directory validation failed: ${e.message || String(e)}`,
        true,
      );
      return;
    }

    const inference = (await applyWorkbenchRootInference({ revertOnConflict: true })) as Inference;
    if (!inference.ok && inference.conflict) {
      setResult(
        'settings-result-directories',
        `Save cancelled: workbench directory and GitHub profile do not match (entered ${inference.existing}, origin inference ${inference.inferred}). workbench_root was not written.`,
        true,
      );
      return;
    }
    if (inference.error) {
      messages.push(`Could not infer GitHub profile: ${inference.error}`);
    }
    if (inference.locked && inference.inferred) {
      includeGithubUrl = true;
      messages.push(`Inferred and locked GitHub profile ${inference.inferred}`);
    }

    const payload: Record<string, string> = {};
    if (includeWorkbenchRoot) {
      payload.workbench_root = workbenchRoot;
    }
    if (includeGithubUrl) {
      payload.github_user_url = githubUserInput.value.trim();
    }
    if (store.notesGithubRepoInferredFromOrigin && isGithubAccountConfigured()) {
      payload.workbench_github_repo_url = store.notesGithubRepoInferredFromOrigin;
    }

    if (!Object.keys(payload).length) {
      return;
    }

    saveBtn.disabled = true;
    saveBtn.textContent = 'Saving…';
    try {
      const resp = await api.setConfig(payload);
      if (resp?.error) throw new Error(resp.error);
      if (payload.github_user_url) {
        setGithubUserUrl(payload.github_user_url);
      }
      const parts: string[] = [];
      if (payload.workbench_root) parts.push('Workbench knowledge directory');
      if (payload.github_user_url) parts.push('GitHub profile');
      if (payload.workbench_github_repo_url) parts.push('Notes GitHub repository');
      let msg = `Saved: ${parts.join(', ')}.`;
      if (messages.length) msg += ` ${messages.join('；')}`;
      setResult('settings-result-directories', msg);
      await loadSettingsSnapshot();
    } catch (err) {
      const e = err as Error;
      setResult('settings-result-directories', `Save failed: ${e.message || String(e)}`, true);
    } finally {
      saveBtn.disabled = false;
      saveBtn.textContent = 'Save';
    }
  });

  btn('btn-settings-save-github')?.addEventListener('click', async () => {
    const saveBtn = btn('btn-settings-save-github');
    const githubInput = input('settings-github-user-url');
    const githubUserUrl = githubInput.value.trim();
    const token = input('settings-github-token').value.trim();
    const locked = isGithubUserUrlInferredLocked();

    const payload: Record<string, string> = {};
    if (!locked) {
      payload.github_user_url = githubUserUrl;
    } else if (store.githubUserUrlInferredFromOrigin) {
      payload.github_user_url = store.githubUserUrlInferredFromOrigin;
    }
    if (token) payload.github_token = token;

    if (!Object.keys(payload).length) {
      if (locked) {
        setResult(
          'settings-result-github',
          `GitHub profile is locked to ${store.githubUserUrlInferredFromOrigin}; no need to save again. To update the Token, enter it and save.`,
          false,
        );
      } else {
        setResult('settings-result-github', 'Enter a GitHub profile URL or Token.', true);
      }
      return;
    }

    saveBtn.disabled = true;
    saveBtn.textContent = 'Saving…';
    try {
      const resp = await api.setConfig(payload);
      if (resp?.error) throw new Error(resp.error);
      if (payload.github_user_url) {
        setGithubUserUrl(githubUserUrl);
        savedSnapshot.githubUserUrl = githubUserUrl;
      }
      const parts: string[] = [];
      if (payload.github_user_url) parts.push('GitHub profile');
      if (payload.github_token) parts.push('Token');
      if (locked && !payload.github_user_url) {
        setResult(
          'settings-result-github',
          `Saved${parts.length ? `：${parts.join('、')}` : ''}. GitHub profile remains inferred value ${store.githubUserUrlInferredFromOrigin}。`,
        );
      } else {
        setResult('settings-result-github', `Saved: ${parts.join(', ')}.`);
      }
      input('settings-github-token').value = '';
      await loadSettingsSnapshot();
    } catch (err) {
      const e = err as Error;
      setResult('settings-result-github', `Save failed: ${e.message || String(e)}`, true);
    } finally {
      saveBtn.disabled = false;
      saveBtn.textContent = 'Save';
    }
  });

  document.getElementById('settings-llm-engine')?.addEventListener('change', (e) => {
    void applyEngineCategorySelection((e.target as HTMLSelectElement).value, { clearCredential: true });
  });

  btn('btn-settings-save-llm')?.addEventListener('click', () => {
    void saveAssistantEnginePanel();
  });

  document.getElementById('btn-settings-mcp-generate')?.addEventListener('click', () => {
    void generateCursorIdeServerBlock();
  });
  document.getElementById('btn-settings-mcp-rotate')?.addEventListener('click', () => {
    void rotateCursorIdeTicket();
  });
  document.getElementById('btn-settings-mcp-revoke')?.addEventListener('click', () => {
    void revokeMcpSlotTicket();
  });
}
