import * as api from '../../api.js';
import { setGithubUserUrl } from '../../constants.js';
import { getKbHidePattern, saveKbHidePattern } from '../../kb-hide-pattern.js';
import { getEnginePreset, listEngineCategories } from './engine-presets.js';

const GITHUB_USER_HINT_DEFAULT =
  'Your GitHub profile URL for Viewer remote links and promotion sources; may be saved with a Token.';

const DEFAULT_ENGINE_CATEGORY = 'host';

/** @type {{ has_host_key: boolean, has_cursor_key: boolean }} */
const engineKeyHints = {
  has_host_key: false,
  has_cursor_key: false,
};

// ── Nav switching ──────────────────────────────────────────────────────────

function switchPanel(panelId) {
  document.querySelectorAll('.settings-nav-item').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.panel === panelId);
  });
  document.querySelectorAll('.settings-panel').forEach(panel => {
    panel.classList.toggle('active', panel.id === `settings-panel-${panelId}`);
  });
}

document.querySelectorAll('.settings-nav-item').forEach(btn => {
  btn.addEventListener('click', async () => {
    const panelId = btn.dataset.panel;
    switchPanel(panelId);
    if (panelId === 'github') {
      await syncGithubUserUrlLockFromWorkbenchRoot();
    }
  });
});

// ── Helpers ────────────────────────────────────────────────────────────────

function setResult(resultElId, message, isError = false) {
  const el = document.getElementById(resultElId);
  if (!el) return;
  el.textContent = message || '';
  el.style.color = isError ? '#cf222e' : '#1a7f37';
}

function normalizeGithubUserUrl(url) {
  return (url || '').trim().replace(/\/$/, '').toLowerCase();
}

/** Last saved values from server — used to revert on inference conflict. */
const savedSnapshot = {
  workbenchKnowledgeRoot: '',
  githubUserUrl: '',
};

/** Origin 推断锁：推断成功后禁止手改 github_user_url。 */
let githubUserUrlInferredFromOrigin = '';

function setGithubUserUrlInferredLock(inferredUrl, locked) {
  const input = document.getElementById('settings-github-user-url');
  const hint = document.getElementById('settings-github-user-hint');
  if (!input || !hint) return;

  if (locked && inferredUrl) {
    githubUserUrlInferredFromOrigin = inferredUrl;
    input.value = inferredUrl;
    input.readOnly = true;
    input.disabled = true;
    input.classList.add('settings-input-readonly');
    setGithubUserUrl(inferredUrl);
    hint.textContent =
      'Inferred from workbench directory git origin (read-only; change the workbench directory or repo remote)';
    hint.style.color = '#1a7f37';
  } else {
    githubUserUrlInferredFromOrigin = '';
    input.readOnly = false;
    input.disabled = false;
    input.classList.remove('settings-input-readonly');
    hint.textContent = GITHUB_USER_HINT_DEFAULT;
    hint.style.color = '';
  }
}

function clearGithubUserUrlInferredLock() {
  setGithubUserUrlInferredLock('', false);
}

function syncKbHidePatternInput() {
  const kbHideInput = document.getElementById('settings-kb-hide-pattern');
  if (kbHideInput) {
    kbHideInput.value = getKbHidePattern();
  }
}

function normalizeEngineCategory(raw) {
  const id = String(raw || '').trim();
  if (id === 'cursor' || id === 'host') return id;
  return DEFAULT_ENGINE_CATEGORY;
}

function ensureEngineCategoryOptions() {
  const select = document.getElementById('settings-llm-engine');
  if (!select) return;
  const categories = listEngineCategories();
  const existing = new Set(
    Array.from(select.options).map((opt) => opt.value),
  );
  for (const cat of categories) {
    if (existing.has(cat.id)) continue;
    const opt = document.createElement('option');
    opt.value = cat.id;
    opt.textContent = cat.label;
    select.appendChild(opt);
  }
}

function credentialHintForCategory(categoryId) {
  const hasKey =
    categoryId === 'cursor'
      ? engineKeyHints.has_cursor_key
      : engineKeyHints.has_host_key;
  return hasKey
    ? 'API key configured. Enter a new key to replace it.'
    : 'No API key configured.';
}

/**
 * Fill Assistant/Engine panel from config (category, readonly preset, model, credential hint).
 * @param {Record<string, unknown>} cfg
 */
function loadAssistantEnginePanel(cfg) {
  ensureEngineCategoryOptions();
  const categoryId = normalizeEngineCategory(cfg?.assistant_engine);
  const preset = getEnginePreset(categoryId) || getEnginePreset(DEFAULT_ENGINE_CATEGORY);
  const llm = cfg?.llm ?? {};

  // Legacy `has_llm_key` maps to host credential (t3 migration).
  engineKeyHints.has_host_key = Boolean(cfg?.has_host_key ?? cfg?.has_llm_key);
  engineKeyHints.has_cursor_key = Boolean(cfg?.has_cursor_key);

  const engineSelect = document.getElementById('settings-llm-engine');
  const platformInput = document.getElementById('settings-llm-platform');
  const baseUrlInput = document.getElementById('settings-llm-base-url');
  const modelInput = document.getElementById('settings-llm-model');
  const keyHint = document.getElementById('settings-llm-key-hint');
  const apiKeyInput = document.getElementById('settings-llm-api-key');

  if (engineSelect) engineSelect.value = categoryId;
  if (platformInput) {
    platformInput.value = preset?.fields?.platform ?? '';
    platformInput.readOnly = true;
    platformInput.classList.add('settings-input-readonly');
  }
  if (baseUrlInput) {
    baseUrlInput.value = preset?.fields?.base_url ?? '';
    baseUrlInput.readOnly = true;
    baseUrlInput.classList.add('settings-input-readonly');
  }
  if (modelInput) {
    modelInput.value = typeof llm.model === 'string' ? llm.model : '';
    modelInput.readOnly = false;
    modelInput.disabled = false;
  }
  if (apiKeyInput) apiKeyInput.value = '';
  if (keyHint) keyHint.textContent = credentialHintForCategory(categoryId);
}

function applyEngineCategorySelection(categoryId, { clearCredential = true } = {}) {
  const id = normalizeEngineCategory(categoryId);
  const preset = getEnginePreset(id) || getEnginePreset(DEFAULT_ENGINE_CATEGORY);
  const engineSelect = document.getElementById('settings-llm-engine');
  const platformInput = document.getElementById('settings-llm-platform');
  const baseUrlInput = document.getElementById('settings-llm-base-url');
  const keyHint = document.getElementById('settings-llm-key-hint');
  const apiKeyInput = document.getElementById('settings-llm-api-key');

  if (engineSelect) engineSelect.value = id;
  if (platformInput) platformInput.value = preset?.fields?.platform ?? '';
  if (baseUrlInput) baseUrlInput.value = preset?.fields?.base_url ?? '';
  if (clearCredential && apiKeyInput) apiKeyInput.value = '';
  if (keyHint) keyHint.textContent = credentialHintForCategory(id);
}

async function saveAssistantEnginePanel() {
  const btn = document.getElementById('btn-settings-save-llm');
  const categoryId = normalizeEngineCategory(
    document.getElementById('settings-llm-engine')?.value,
  );
  const model = document.getElementById('settings-llm-model')?.value.trim() ?? '';
  const apiKey = document.getElementById('settings-llm-api-key')?.value.trim() ?? '';

  const payload = {
    assistant_engine: categoryId,
    llm: { model },
  };
  if (apiKey) {
    if (categoryId === 'cursor') payload.api_key_cursor = apiKey;
    else payload.api_key_host = apiKey;
  }

  btn.disabled = true;
  btn.textContent = 'Saving…';
  try {
    const resp = await api.setConfig(payload);
    if (resp?.error) throw new Error(resp.error);
    const parts = ['Engine', 'Model'];
    if (apiKey) parts.push('Credential');
    setResult('settings-result-llm', `Saved: ${parts.join(', ')}.`);
    document.getElementById('settings-llm-api-key').value = '';
    await loadSettingsSnapshot();
  } catch (e) {
    setResult('settings-result-llm', `Save failed: ${e.message || String(e)}`, true);
  } finally {
    btn.disabled = false;
    btn.textContent = 'Save';
  }
}

function isGithubUserUrlInferredLocked() {
  return Boolean(githubUserUrlInferredFromOrigin);
}

/**
 * 工作台目录与 origin 推断一致时，打开设置即锁定主页输入框。
 */
async function syncGithubUserUrlLockFromWorkbenchRoot() {
  const archiveInput = document.getElementById('settings-archive-root');
  const githubInput = document.getElementById('settings-github-user-url');
  const root = archiveInput?.value.trim() ?? '';
  if (!root) {
    clearGithubUserUrlInferredLock();
    return;
  }

  let resp;
  try {
    resp = await api.inferGithubUserUrl(root);
  } catch (e) {
    clearGithubUserUrlInferredLock();
    setResult(
      'settings-result-github',
      `Could not infer GitHub profile: ${e.message || String(e)}. If you just updated the app, fully restart and try again.`,
      true,
    );
    return;
  }

  const inferred = (resp?.github_user_url || '').trim();
  if (!inferred) {
    clearGithubUserUrlInferredLock();
    setResult(
      'settings-result-github',
      'No git origin detected; could not infer GitHub profile — enter it manually.',
      false,
    );
    return;
  }

  const current = githubInput?.value.trim() ?? '';
  if (!current || normalizeGithubUserUrl(current) === normalizeGithubUserUrl(inferred)) {
    setGithubUserUrlInferredLock(inferred, true);
    setResult('settings-result-github', '');
  } else {
    clearGithubUserUrlInferredLock();
    setResult(
      'settings-result-github',
      `Entered ${current} does not match origin inference ${inferred}; clear the field or change the workbench directory, then retry.`,
      true,
    );
  }
}

/**
 * Infer github_user_url from workbench root (git origin).
 * @returns {Promise<{ ok: boolean, conflict?: boolean, autofilled?: boolean, inferred?: string, existing?: string, noRemote?: boolean, locked?: boolean }>}
 */
async function applyWorkbenchRootInference({ revertOnConflict = true } = {}) {
  const archiveInput = document.getElementById('settings-archive-root');
  const githubInput = document.getElementById('settings-github-user-url');
  const root = archiveInput.value.trim();
  if (!root) {
    clearGithubUserUrlInferredLock();
    return { ok: true };
  }

  let resp;
  try {
    resp = await api.inferGithubUserUrl(root);
  } catch (e) {
    clearGithubUserUrlInferredLock();
    return { ok: true, noRemote: true, error: e.message || String(e) };
  }

  const inferred = (resp?.github_user_url || '').trim();
  if (!inferred) {
    clearGithubUserUrlInferredLock();
    return { ok: true, noRemote: true };
  }

  const existing = githubInput.value.trim();
  const existingNorm = normalizeGithubUserUrl(existing);
  const inferredNorm = normalizeGithubUserUrl(inferred);

  if (existing && existingNorm !== inferredNorm) {
    if (revertOnConflict) {
      archiveInput.value = savedSnapshot.workbenchKnowledgeRoot;
    }
    clearGithubUserUrlInferredLock();
    return { ok: false, conflict: true, existing, inferred };
  }

  setGithubUserUrlInferredLock(inferred, true);
  return {
    ok: true,
    autofilled: !existing,
    inferred,
    locked: true,
  };
}

// ── Load snapshot ──────────────────────────────────────────────────────────

async function loadSettingsSnapshot() {
  try {
    const cfg = await api.fetchConfig();

    const archiveInput = document.getElementById('settings-archive-root');
    const kbInput = document.getElementById('settings-kb-root');
    const githubUserInput = document.getElementById('settings-github-user-url');
    const wbRoot = cfg?.workbench_knowledge_root ?? '';
    const ghUrl = cfg?.github_user_url ?? '';
    if (wbRoot) {
      archiveInput.placeholder = wbRoot;
      archiveInput.value = wbRoot;
    }
    if (cfg?.knowledge_corpus_root) {
      kbInput.placeholder = cfg.knowledge_corpus_root;
      kbInput.value = cfg.knowledge_corpus_root;
    }
    clearGithubUserUrlInferredLock();
    if (githubUserInput) {
      githubUserInput.value = ghUrl;
    }
    setGithubUserUrl(ghUrl);
    savedSnapshot.workbenchKnowledgeRoot = wbRoot;
    savedSnapshot.githubUserUrl = ghUrl;

    const hintEl = document.getElementById('settings-token-hint');
    hintEl.textContent = cfg?.has_github_token
      ? 'GitHub Token configured. Enter a new token to replace it.'
      : 'No GitHub Token configured.';

    loadAssistantEnginePanel(cfg ?? {});

    await syncGithubUserUrlLockFromWorkbenchRoot();
    syncKbHidePatternInput();
  } catch {
    document.getElementById('settings-token-hint').textContent =
      'Could not load settings; you can type and save.';
    document.getElementById('settings-llm-key-hint').textContent =
      'Could not load settings; you can type and save.';
    loadAssistantEnginePanel({});
    clearGithubUserUrlInferredLock();
    syncKbHidePatternInput();
  }
}

// ── Open / close ───────────────────────────────────────────────────────────

export async function openSettingsDialog() {
  setResult('settings-result-directories', '');
  setResult('settings-result-github', '');
  setResult('settings-result-knowledge', '');
  setResult('settings-result-llm', '');
  document.getElementById('settings-github-token').value = '';
  document.getElementById('settings-llm-api-key').value = '';
  switchPanel('directories');
  await loadSettingsSnapshot();
  document.getElementById('settings-dialog').classList.add('open');
}

function closeSettingsDialog() {
  document.getElementById('settings-dialog').classList.remove('open');
}

document.getElementById('btn-settings-close').addEventListener('click', closeSettingsDialog);
document.getElementById('btn-settings-cancel').addEventListener('click', closeSettingsDialog);
document.getElementById('settings-dialog').addEventListener('click', (e) => {
  if (e.target === document.getElementById('settings-dialog')) closeSettingsDialog();
});

document.getElementById('settings-archive-root').addEventListener('input', () => {
  const root = document.getElementById('settings-archive-root').value.trim();
  if (!root) {
    clearGithubUserUrlInferredLock();
  }
});

// ── Workbench root blur: infer GitHub profile ─────────────────────────────────

document.getElementById('settings-archive-root').addEventListener('blur', async () => {
  const root = document.getElementById('settings-archive-root').value.trim();
  if (!root || root === savedSnapshot.workbenchKnowledgeRoot) {
    return;
  }
  const inference = await applyWorkbenchRootInference({ revertOnConflict: true });
  if (!inference.ok && inference.conflict) {
    setResult(
      'settings-result-directories',
      `Directory not saved: GitHub profile on the GitHub tab (${inference.existing}) does not match origin inference (${inference.inferred}). Fix or clear the profile on the GitHub tab before changing the directory.`,
      true,
    );
    return;
  }
  if (inference.locked && inference.inferred) {
    setResult(
      'settings-result-directories',
      `GitHub profile inferred from git origin (locked — save on the GitHub tab).`,
      false,
    );
    setResult('settings-result-github', '');
    switchPanel('github');
    return;
  }
  if (inference.noRemote) {
    setResult(
      'settings-result-directories',
      'No git origin detected; could not auto-infer GitHub profile — enter it manually on the GitHub tab.',
      false,
    );
  }
});

// ── Save: 目录配置 ──────────────────────────────────────────────────────────

document.getElementById('btn-settings-save-directories').addEventListener('click', async () => {
  const btn = document.getElementById('btn-settings-save-directories');
  const archiveInput = document.getElementById('settings-archive-root');
  const workbenchKnowledgeRoot = archiveInput.value.trim();
  const knowledgeCorpusRoot = document.getElementById('settings-kb-root').value.trim();
  const githubUserInput = document.getElementById('settings-github-user-url');

  if (!workbenchKnowledgeRoot && !knowledgeCorpusRoot) {
    setResult('settings-result-directories', 'Enter at least one directory path.', true);
    return;
  }

  let includeWorkbenchRoot = Boolean(workbenchKnowledgeRoot);
  let includeGithubUrl = false;
  const messages = [];

  if (workbenchKnowledgeRoot) {
    try {
      const check = await api.checkWorkbenchKnowledgeRoot(workbenchKnowledgeRoot);
      if (check?.ok === false) {
        setResult(
          'settings-result-directories',
          check.error || 'Workbench directory invalid; not saved.',
          true,
        );
        includeWorkbenchRoot = false;
        if (!knowledgeCorpusRoot) return;
      }
    } catch (e) {
      setResult(
        'settings-result-directories',
        `Workbench directory validation failed: ${e.message || String(e)}`,
        true,
      );
      includeWorkbenchRoot = false;
      if (!knowledgeCorpusRoot) return;
    }
  }

  if (workbenchKnowledgeRoot && includeWorkbenchRoot) {
    const inference = await applyWorkbenchRootInference({ revertOnConflict: true });
    if (!inference.ok && inference.conflict) {
      setResult(
        'settings-result-directories',
        `Save cancelled: workbench directory and GitHub profile do not match (entered ${inference.existing}, origin inference ${inference.inferred}). workbench_knowledge_root was not written.`,
        true,
      );
      includeWorkbenchRoot = false;
      if (!knowledgeCorpusRoot) {
        return;
      }
    } else if (inference.locked && inference.inferred) {
      includeGithubUrl = true;
      messages.push(`Inferred and locked GitHub profile ${inference.inferred}`);
    }
  }

  const payload = {};
  if (includeWorkbenchRoot) {
    payload.workbench_knowledge_root = workbenchKnowledgeRoot;
  }
  if (knowledgeCorpusRoot) {
    payload.knowledge_corpus_root = knowledgeCorpusRoot;
  }
  if (includeGithubUrl) {
    payload.github_user_url = githubUserInput.value.trim();
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
    if (payload.knowledge_corpus_root) parts.push('Knowledge corpus directory');
    if (payload.github_user_url) parts.push('GitHub profile');
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

// ── Save: 知识库 hide pattern ───────────────────────────────────────────────

document.getElementById('btn-settings-save-knowledge').addEventListener('click', () => {
  const btn = document.getElementById('btn-settings-save-knowledge');
  const pattern = document.getElementById('settings-kb-hide-pattern').value;
  btn.disabled = true;
  btn.textContent = 'Saving…';
  try {
    const result = saveKbHidePattern(pattern);
    if (!result.ok) {
      setResult('settings-result-knowledge', `Invalid regex: ${result.error}`, true);
      return;
    }
    setResult('settings-result-knowledge', 'Hide rules saved.');
  } finally {
    btn.disabled = false;
    btn.textContent = 'Save';
  }
});

// ── Save: GitHub profile + Token ───────────────────────────────────────────────

document.getElementById('btn-settings-save-github').addEventListener('click', async () => {
  const btn = document.getElementById('btn-settings-save-github');
  const githubInput = document.getElementById('settings-github-user-url');
  const githubUserUrl = githubInput.value.trim();
  const token = document.getElementById('settings-github-token').value.trim();
  const locked = isGithubUserUrlInferredLocked();

  const payload = {};
  if (!locked) {
    payload.github_user_url = githubUserUrl;
  }
  if (token) payload.github_token = token;

  if (!Object.keys(payload).length) {
    if (locked) {
      setResult(
        'settings-result-github',
        `GitHub profile is locked to ${githubUserUrlInferredFromOrigin}; no need to save again. To update the Token, enter it and save.`,
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
        `Saved${parts.length ? `：${parts.join('、')}` : ''}. GitHub profile remains inferred value ${githubUserUrlInferredFromOrigin}。`,
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

// ── Save: Assistant / Engine (category + credential + model) ────────────────

document.getElementById('settings-llm-engine')?.addEventListener('change', (e) => {
  applyEngineCategorySelection(e.target.value, { clearCredential: true });
});

document.getElementById('btn-settings-save-llm').addEventListener('click', () => {
  void saveAssistantEnginePanel();
});
