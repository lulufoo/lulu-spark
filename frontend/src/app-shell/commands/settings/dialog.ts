import * as api from '../../../host/api.ts';
import { setGithubUserUrl } from '../../../host/constants.ts';
import { state } from '../../../host/state.ts';
import { loadKbHidePatterns } from './kb-hide-patterns.ts';
import { applyEngineCategorySelection, saveAssistantEnginePanel } from './engine.ts';
import {
  applyWorkbenchRootInference,
  clearGithubUserUrlInferredLock,
  syncGithubUserUrlLockFromWorkbenchRoot,
} from './github-user.ts';
import {
  deselectAllMcpChannelTools,
  loadMcpChannelTools,
  paintMcpToolGroups,
  persistMcpChannelTools,
  selectAllMcpChannelTools,
} from './mcp.ts';
import {
  clearMcpServerBlock,
  copyCursorIdeServerBlock,
  expireMobileDevice,
  expireWorkbenchTicket,
  loadMcpTicketView,
  runCursorIdePrimaryAction,
} from './mcp-tickets.ts';
import {
  addWorkbenchGithubRepo,
  isGithubAccountConfigured,
} from './workbench-github.ts';
import {
  isGithubUserUrlInferredLocked,
  savedSnapshot,
  setResult,
  store,
} from '../../state/settings/store.ts';
import { settingsOpenStore } from '../../state/dialog-open.ts';
import {
  clearNotesCategoryError,
  closeNotesCategoryEditor,
  loadNotesCategories,
} from './notes-categories.ts';
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
  closeNotesCategoryEditor();
  settingsOpenStore.set(false);
  document.getElementById('settings-dialog')?.classList.remove('open');
}

export async function openSettingsDialog(opts: SettingsOpenOpts = {}) {
  closeNotesCategoryEditor();
  settingsOpenStore.set(true);
  document.getElementById('settings-dialog')?.classList.add('open');
  ensureWired();
  setResult('settings-result-workbench', '');
  setResult('workbench-connect-error', '');
  setResult('knowledge-root-error', '');
  setResult('settings-result-knowledge', '');
  setResult('settings-result-github', '');
  setResult('settings-result-llm', '');
  setResult('settings-result-mcp', '');
  setResult('settings-result-mcp-tools', '');
  clearMcpServerBlock();
  const tokenInput = input('settings-github-token');
  const keyInput = input('settings-llm-api-key');
  if (tokenInput) tokenInput.value = '';
  if (keyInput) keyInput.value = '';
  const panelId = opts.panel || 'workbench';
  switchSettingsTab('workbench', panelId === 'workbench' ? (opts.tab || 'directory') : 'directory');
  switchSettingsTab('notes', panelId === 'notes' ? (opts.tab || 'add') : 'add');
  switchSettingsTab('knowledge', panelId === 'knowledge' ? (opts.tab || 'directory') : 'directory');
  switchSettingsTab('llm', 'engine');
  switchSettingsTab('github', 'account');
  switchSettingsTab('mcp', panelId === 'mcp' ? (opts.tab || 'tickets') : 'tickets');
  switchPanel(panelId);
  await loadSettingsSnapshot();
  await loadKbHidePatterns();
  await loadNotesCategories();
  await loadMcpChannelTools();
  requestAnimationFrame(() => paintMcpToolGroups());
  await loadMcpTicketView();
}

function wireSettingsDialog() {
  document.querySelectorAll('.settings-tab').forEach((tabBtn) => {
    tabBtn.addEventListener('click', () => {
      const panel = tabBtn.closest('.settings-panel');
      const panelId = panel?.id?.replace(/^settings-panel-/, '');
      const tab = (tabBtn as HTMLElement).dataset.tab;
      if (panelId !== 'notes' || tab !== 'edit') closeNotesCategoryEditor();
      if (panelId === 'notes') clearNotesCategoryError();
      if (panelId && tab) switchSettingsTab(panelId, tab);
      if (panelId === 'mcp' && tab === 'tools') void loadMcpChannelTools();
    });
  });

  document.querySelectorAll('.settings-nav-item').forEach((navBtn) => {
    navBtn.addEventListener('click', async () => {
      const panelId = (navBtn as HTMLElement).dataset.panel;
      if (!panelId) return;
      if (panelId !== 'notes') closeNotesCategoryEditor();
      switchPanel(panelId);
      if (panelId === 'github') {
        await syncGithubUserUrlLockFromWorkbenchRoot();
      }
    });
  });

  btn('btn-settings-close')?.addEventListener('click', closeSettingsDialog);
  document.getElementById('settings-dialog')?.addEventListener('click', (e) => {
    if (e.target === document.getElementById('settings-dialog')) closeSettingsDialog();
  });
  document.getElementById('btn-workbench-connect-add')?.addEventListener('click', () => {
    void addWorkbenchGithubRepo();
  });
  document.getElementById('workbench-connect-url')?.addEventListener('keydown', (e) => {
    if ((e as KeyboardEvent).key === 'Enter') document.getElementById('btn-workbench-connect-add')?.click();
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
  input('settings-workbench-root')?.addEventListener('input', () => {
    const root = input('settings-workbench-root').value.trim();
    if (!root) {
      clearGithubUserUrlInferredLock();
    }
  });

  input('settings-workbench-root')?.addEventListener('blur', async () => {
    const root = input('settings-workbench-root').value.trim();
    if (!root) {
      return;
    }
    const pathChanged = root !== savedSnapshot.workbenchRoot;
    const inference = (await applyWorkbenchRootInference({ revertOnConflict: pathChanged })) as Inference;
    if (!inference.ok && inference.conflict) {
      setResult(
        'settings-result-workbench',
        `Directory not saved: GitHub profile on Sync (${inference.existing}) does not match origin inference (${inference.inferred}). Fix or clear the profile on Sync before changing the directory.`,
        true,
      );
      return;
    }
    if (inference.error) {
      setResult(
        'settings-result-workbench',
        `Could not infer GitHub profile: ${inference.error}. If you just updated the app, fully restart and try again.`,
        true,
      );
      return;
    }
    if (inference.locked && inference.inferred) {
      setResult(
        'settings-result-workbench',
        `GitHub profile inferred from git origin (locked — save on Sync).`,
        false,
      );
      setResult('settings-result-github', '');
      if (pathChanged) switchPanel('workbench');
      return;
    }
    if (inference.noRemote) {
      setResult(
        'settings-result-workbench',
        'No git origin detected; could not auto-infer GitHub profile — enter it manually on Sync.',
        false,
      );
    }
  });

  btn('btn-settings-save-workbench')?.addEventListener('click', async () => {
    const saveBtn = btn('btn-settings-save-workbench');
    const workbenchInput = input('settings-workbench-root');
    const workbenchRoot = workbenchInput.value.trim();
    const githubUserInput = input('settings-github-user-url');

    if (!workbenchRoot) {
      setResult('settings-result-workbench', 'Enter a directory path.', true);
      return;
    }

    const includeWorkbenchRoot = true;
    let includeGithubUrl = false;
    const messages: string[] = [];

    try {
      const check = await api.checkWorkbenchRoot(workbenchRoot);
      if (check?.ok === false) {
        setResult(
          'settings-result-workbench',
          check.error || 'Data directory invalid; not saved.',
          true,
        );
        return;
      }
    } catch (err) {
      const e = err as Error;
      setResult(
        'settings-result-workbench',
        `Data directory validation failed: ${e.message || String(e)}`,
        true,
      );
      return;
    }

    const inference = (await applyWorkbenchRootInference({ revertOnConflict: true })) as Inference;
    if (!inference.ok && inference.conflict) {
      setResult(
        'settings-result-workbench',
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
    if (store.workbenchGithubRepoInferredFromOrigin && isGithubAccountConfigured()) {
      payload.workbench_github_repo_url = store.workbenchGithubRepoInferredFromOrigin;
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
      if (payload.workbench_root) parts.push('Data directory');
      if (payload.github_user_url) parts.push('GitHub profile');
      if (payload.workbench_github_repo_url) parts.push('Data-store GitHub repository');
      let msg = `Saved: ${parts.join(', ')}.`;
      if (messages.length) msg += ` ${messages.join('；')}`;
      setResult('settings-result-workbench', msg);
      await loadSettingsSnapshot();
    } catch (err) {
      const e = err as Error;
      setResult('settings-result-workbench', `Save failed: ${e.message || String(e)}`, true);
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

  document.getElementById('btn-settings-mcp-primary')?.addEventListener('click', () => {
    void runCursorIdePrimaryAction();
  });
  document.getElementById('btn-settings-mcp-copy')?.addEventListener('click', () => {
    void copyCursorIdeServerBlock();
  });
  document.getElementById('btn-settings-mcp-workbench-expire')?.addEventListener('click', () => {
    void expireWorkbenchTicket();
  });
  document.getElementById('settings-mcp-device-list')?.addEventListener('click', (event) => {
    const target = event.target as HTMLElement | null;
    const deviceId = target?.closest('button')?.dataset.mcpDeviceExpire;
    if (deviceId) void expireMobileDevice(deviceId);
  });
  document.getElementById('settings-mcp-ticket-channel')?.addEventListener('change', () => {
    void loadMcpTicketView();
  });
  document.getElementById('settings-mcp-channel')?.addEventListener('change', () => {
    paintMcpToolGroups();
  });
  document.getElementById('settings-mcp-tool-groups')?.addEventListener('change', (event) => {
    const target = event.target as HTMLElement | null;
    if (target?.matches?.('input[data-mcp-tool]')) void persistMcpChannelTools();
  });
  document.getElementById('btn-settings-mcp-tools-select-all')?.addEventListener('click', () => {
    void selectAllMcpChannelTools();
  });
  document.getElementById('btn-settings-mcp-tools-deselect-all')?.addEventListener('click', () => {
    void deselectAllMcpChannelTools();
  });
}
