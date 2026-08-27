import * as api from '../host/api.js';
import { setGithubUserUrl } from '../host/constants.js';
import { saveKbHidePattern } from '../corpus/corpus-hide-pattern.js';
import { state } from '../host/state.js';
import { applyEngineCategorySelection, saveAssistantEnginePanel } from './settings/engine.js';
import {
  applyWorkbenchRootInference,
  clearGithubUserUrlInferredLock,
  syncGithubUserUrlLockFromWorkbenchRoot,
} from './settings/github-user.js';
import {
  clearMcpServerBlock,
  generateCursorIdeServerBlock,
  rotateCursorIdeTicket,
  revokeMcpSlotTicket,
} from './settings/mcp.js';
import {
  addNotesGithubRepo,
  isGithubAccountConfigured,
} from './settings/notes-github.js';
import {
  isGithubUserUrlInferredLocked,
  savedSnapshot,
  setResult,
  store,
} from './settings/store.js';
import { loadSettingsSnapshot } from './settings/snapshot.js';
import { switchPanel, switchSettingsTab } from './settings/tabs.js';

export async function openSettingsDialog(opts = {}) {
  setResult('settings-result-directories', '');
  setResult('notes-connect-error', '');
  setResult('sediment-kb-corpus-error', '');
  setResult('settings-result-knowledge', '');
  setResult('settings-result-github', '');
  setResult('settings-result-llm', '');
  setResult('settings-result-mcp', '');
  clearMcpServerBlock();
  document.getElementById('settings-github-token').value = '';
  document.getElementById('settings-llm-api-key').value = '';
  const panelId = opts.panel || 'directories';
  switchSettingsTab('directories', panelId === 'directories' ? (opts.tab || 'directory') : 'directory');
  switchSettingsTab('knowledge', panelId === 'knowledge' ? (opts.tab || 'directory') : 'directory');
  switchSettingsTab('llm', 'engine');
  switchSettingsTab('github', 'account');
  switchPanel(panelId);
  await loadSettingsSnapshot();
  document.getElementById('settings-dialog').classList.add('open');
}

function closeSettingsDialog() {
  document.getElementById('settings-dialog').classList.remove('open');
}

document.querySelectorAll('.settings-tab').forEach((btn) => {
  btn.addEventListener('click', () => {
    const panel = btn.closest('.settings-panel');
    const panelId = panel?.id?.replace(/^settings-panel-/, '');
    if (panelId) switchSettingsTab(panelId, btn.dataset.tab);
  });
});

document.querySelectorAll('.settings-nav-item').forEach(btn => {
  btn.addEventListener('click', async () => {
    const panelId = btn.dataset.panel;
    switchPanel(panelId);
    if (panelId === 'github') {
      await syncGithubUserUrlLockFromWorkbenchRoot();
    }
  });
});

document.getElementById('btn-settings-close').addEventListener('click', closeSettingsDialog);
document.getElementById('btn-settings-cancel').addEventListener('click', closeSettingsDialog);
document.getElementById('settings-dialog').addEventListener('click', (e) => {
  if (e.target === document.getElementById('settings-dialog')) closeSettingsDialog();
});
document.getElementById('btn-notes-connect-add')?.addEventListener('click', () => {
  void addNotesGithubRepo();
});
document.getElementById('notes-connect-url')?.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') document.getElementById('btn-notes-connect-add')?.click();
});
document.getElementById('btn-sediment-kb-corpus-save')?.addEventListener('click', () => {
  void (async () => {
    const input = document.getElementById('sediment-kb-corpus-path');
    const saveBtn = document.getElementById('btn-sediment-kb-corpus-save');
    const knowledgeCorpusRoot = input?.value.trim() || '';
    if (!knowledgeCorpusRoot) {
      setResult('sediment-kb-corpus-error', 'Enter a directory path.', true);
      return;
    }
    if (saveBtn) saveBtn.disabled = true;
    setResult('sediment-kb-corpus-error', '');
    try {
      const resp = await api.setConfig({ knowledge_corpus_root: knowledgeCorpusRoot });
      if (resp?.error) throw new Error(resp.error);
      state.ui.knowledgeCorpusRoot = knowledgeCorpusRoot;
      setResult('sediment-kb-corpus-error', 'Saved: Knowledge corpus directory.');
    } catch (e) {
      setResult('sediment-kb-corpus-error', e.message || String(e), true);
    } finally {
      if (saveBtn) saveBtn.disabled = false;
    }
  })();
});
document.getElementById('sediment-kb-corpus-path')?.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') document.getElementById('btn-sediment-kb-corpus-save')?.click();
});
document.getElementById('btn-settings-save-knowledge')?.addEventListener('click', () => {
  const btn = document.getElementById('btn-settings-save-knowledge');
  const pattern = document.getElementById('settings-kb-hide-pattern')?.value ?? '';
  if (btn) btn.disabled = true;
  setResult('settings-result-knowledge', '');
  try {
    const result = saveKbHidePattern(pattern);
    if (!result.ok) {
      setResult('settings-result-knowledge', `Invalid regex: ${result.error}`, true);
      return;
    }
    setResult('settings-result-knowledge', 'Hide rules saved.');
  } finally {
    if (btn) btn.disabled = false;
  }
});
document.getElementById('settings-kb-hide-pattern')?.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') document.getElementById('btn-settings-save-knowledge')?.click();
});

document.getElementById('settings-archive-root').addEventListener('input', () => {
  const root = document.getElementById('settings-archive-root').value.trim();
  if (!root) {
    clearGithubUserUrlInferredLock();
  }
});

document.getElementById('settings-archive-root').addEventListener('blur', async () => {
  const root = document.getElementById('settings-archive-root').value.trim();
  if (!root) {
    return;
  }
  const pathChanged = root !== savedSnapshot.workbenchKnowledgeRoot;
  const inference = await applyWorkbenchRootInference({ revertOnConflict: pathChanged });
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

document.getElementById('btn-settings-save-directories').addEventListener('click', async () => {
  const btn = document.getElementById('btn-settings-save-directories');
  const archiveInput = document.getElementById('settings-archive-root');
  const workbenchKnowledgeRoot = archiveInput.value.trim();
  const githubUserInput = document.getElementById('settings-github-user-url');

  if (!workbenchKnowledgeRoot) {
    setResult('settings-result-directories', 'Enter a directory path.', true);
    return;
  }

  let includeWorkbenchRoot = true;
  let includeGithubUrl = false;
  const messages = [];

  try {
    const check = await api.checkWorkbenchKnowledgeRoot(workbenchKnowledgeRoot);
    if (check?.ok === false) {
      setResult(
        'settings-result-directories',
        check.error || 'Workbench directory invalid; not saved.',
        true,
      );
      return;
    }
  } catch (e) {
    setResult(
      'settings-result-directories',
      `Workbench directory validation failed: ${e.message || String(e)}`,
      true,
    );
    return;
  }

  const inference = await applyWorkbenchRootInference({ revertOnConflict: true });
  if (!inference.ok && inference.conflict) {
    setResult(
      'settings-result-directories',
      `Save cancelled: workbench directory and GitHub profile do not match (entered ${inference.existing}, origin inference ${inference.inferred}). workbench_knowledge_root was not written.`,
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

  const payload = {};
  if (includeWorkbenchRoot) {
    payload.workbench_knowledge_root = workbenchKnowledgeRoot;
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

  btn.disabled = true;
  btn.textContent = 'Saving…';
  try {
    const resp = await api.setConfig(payload);
    if (resp?.error) throw new Error(resp.error);
    if (payload.github_user_url) {
      setGithubUserUrl(payload.github_user_url);
    }
    const parts = [];
    if (payload.workbench_knowledge_root) parts.push('Workbench knowledge directory');
    if (payload.github_user_url) parts.push('GitHub profile');
    if (payload.workbench_github_repo_url) parts.push('Notes GitHub repository');
    let msg = `Saved: ${parts.join(', ')}.`;
    if (messages.length) msg += ` ${messages.join('；')}`;
    setResult('settings-result-directories', msg);
    await loadSettingsSnapshot();
  } catch (e) {
    setResult('settings-result-directories', `Save failed: ${e.message || String(e)}`, true);
  } finally {
    btn.disabled = false;
    btn.textContent = 'Save';
  }
});

document.getElementById('btn-settings-save-github').addEventListener('click', async () => {
  const btn = document.getElementById('btn-settings-save-github');
  const githubInput = document.getElementById('settings-github-user-url');
  const githubUserUrl = githubInput.value.trim();
  const token = document.getElementById('settings-github-token').value.trim();
  const locked = isGithubUserUrlInferredLocked();

  const payload = {};
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

  btn.disabled = true;
  btn.textContent = 'Saving…';
  try {
    const resp = await api.setConfig(payload);
    if (resp?.error) throw new Error(resp.error);
    if (payload.github_user_url) {
      setGithubUserUrl(githubUserUrl);
      savedSnapshot.githubUserUrl = githubUserUrl;
    }
    const parts = [];
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
    document.getElementById('settings-github-token').value = '';
    await loadSettingsSnapshot();
  } catch (e) {
    setResult('settings-result-github', `Save failed: ${e.message || String(e)}`, true);
  } finally {
    btn.disabled = false;
    btn.textContent = 'Save';
  }
});

document.getElementById('settings-llm-engine')?.addEventListener('change', (e) => {
  void applyEngineCategorySelection(e.target.value, { clearCredential: true });
});

document.getElementById('btn-settings-save-llm').addEventListener('click', () => {
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
