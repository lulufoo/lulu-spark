import * as api from '../../api.js';
import { setGithubUserUrl } from '../../constants.js';
import { getKbHidePattern, saveKbHidePattern } from '../../kb-hide-pattern.js';
import { state } from '../../state.js';
import { escHtml } from '../../utils.js';
import { getEnginePreset, listEngineCategories } from './engine-presets.js';

const GITHUB_USER_HINT_DEFAULT =
  'Inferred from the Notes directory origin when possible; used for Viewer remote links.';
const NOTES_CONNECT_NEEDS_ACCOUNT =
  'Set a Sync token first to bind a Notes repository.';

const DEFAULT_ENGINE_CATEGORY = 'host';

/** @type {{ has_host_key: boolean }} */
const engineKeyHints = {
  has_host_key: false,
};

/** Host model draft (`undefined` = not loaded for this panel session). */
/** @type {{ host: string|undefined }} */
const engineModelByCategory = {
  host: undefined,
};

/** @type {'host'} */
let activeEngineCategory = DEFAULT_ENGINE_CATEGORY;

/** Host MCP listen port (same value GET /health uses in its mcp template). */
let mcpPort = 9876;

// ── Nav switching ──────────────────────────────────────────────────────────

function emitKnowledgeTab(tabId) {
  window.dispatchEvent(new CustomEvent('settings-knowledge-tab', { detail: tabId }));
}

function switchPanel(panelId) {
  document.querySelectorAll('.settings-nav-item').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.panel === panelId);
  });
  document.querySelectorAll('.settings-panel').forEach(panel => {
    panel.classList.toggle('active', panel.id === `settings-panel-${panelId}`);
  });
  if (panelId === 'knowledge') {
    const active = document.querySelector('#settings-panel-knowledge .settings-tab.active');
    emitKnowledgeTab(active?.dataset.tab || 'directory');
  }
}

function switchSettingsTab(panelId, tabId) {
  const root = document.getElementById(`settings-panel-${panelId}`);
  if (!root || !tabId) return;
  root.querySelectorAll('.settings-tab').forEach((btn) => {
    btn.classList.toggle('active', btn.dataset.tab === tabId);
  });
  root.querySelectorAll('.settings-tab-panel').forEach((panel) => {
    panel.classList.toggle('active', panel.dataset.tab === tabId);
  });
  if (
    panelId === 'knowledge' &&
    document.getElementById('settings-panel-knowledge')?.classList.contains('active')
  ) {
    emitKnowledgeTab(tabId);
  }
}

function switchNotesTab(tabId) {
  switchSettingsTab('directories', tabId);
}

function normalizeNotesGithubRepoUrl(raw) {
  const s = (raw || '').trim().replace(/\/+$/, '').replace(/\.git$/i, '');
  if (!s) return '';
  let ownerRepo = '';
  const https = s.match(/^https?:\/\/github\.com\/([^/]+)\/([^/#?]+)/i);
  const ssh = s.match(/^git@github\.com:([^/]+)\/([^/#?]+)/i);
  if (https) ownerRepo = `${https[1]}/${https[2]}`;
  else if (ssh) ownerRepo = `${ssh[1]}/${ssh[2]}`;
  else if (/^[^/\s]+\/[^/\s]+$/.test(s)) ownerRepo = s;
  else return '';
  ownerRepo = ownerRepo.replace(/\.git$/i, '');
  return `https://github.com/${ownerRepo}`;
}

function notesGithubRepoFullName(url) {
  const m = String(url || '').match(/^https:\/\/github\.com\/([^/]+\/[^/]+)/i);
  return m ? m[1] : url;
}

function renderNotesConnection(repoUrl, { locked = false } = {}) {
  const addWrap = document.getElementById('notes-connect-add');
  const item = document.getElementById('notes-connect-item');
  const url = (repoUrl || '').trim();
  if (addWrap) addWrap.hidden = Boolean(url);
  if (!item) return;
  if (!url) {
    item.innerHTML = '';
    return;
  }
  const fullName = notesGithubRepoFullName(url);
  const deleteBtn = locked
    ? ''
    : `<button type="button" class="sediment-kb-delete-btn" id="btn-notes-connect-delete">Delete</button>`;
  item.innerHTML = `<div class="repo-list-item">
      <div class="repo-list-item-info">
        <div class="repo-list-item-name">${escHtml(fullName)}</div>
        <div class="repo-list-item-desc">${escHtml(url)}</div>
      </div>
      <div class="repo-list-item-actions">
        <a class="repo-list-item-link" href="${escHtml(url)}" target="_blank" rel="noopener noreferrer">Link ↗</a>
        ${deleteBtn}
      </div>
    </div>`;
  if (!locked) {
    document.getElementById('btn-notes-connect-delete')?.addEventListener('click', () => {
      void deleteNotesGithubRepo();
    });
  }
}

function applyNotesGithubRepoFromInferResponse(resp) {
  const inferred = normalizeNotesGithubRepoUrl(resp?.workbench_github_repo_url || '');
  if (inferred && !isGithubAccountConfigured()) {
    notesGithubRepoInferredFromOrigin = '';
    renderNotesConnection(savedSnapshot.workbenchGithubRepoUrl, { locked: false });
    setResult('notes-connect-error', NOTES_CONNECT_NEEDS_ACCOUNT, true);
    syncNotesConnectionAccess();
    return;
  }
  notesGithubRepoInferredFromOrigin = inferred;
  if (inferred) {
    renderNotesConnection(inferred, { locked: true });
    setResult(
      'notes-connect-error',
      'Inferred from workbench directory git origin (read-only).',
    );
    return;
  }
  renderNotesConnection(savedSnapshot.workbenchGithubRepoUrl, { locked: false });
  setResult('notes-connect-error', '');
}

async function saveNotesGithubRepoUrl(repoUrl) {
  const resp = await api.setConfig({ workbench_github_repo_url: repoUrl });
  if (resp?.error) throw new Error(resp.error);
  savedSnapshot.workbenchGithubRepoUrl = repoUrl;
  renderNotesConnection(repoUrl);
}

function isGithubAccountConfigured() {
  if (savedSnapshot.hasGithubToken) return true;
  const n = normalizeGithubUserUrl(savedSnapshot.githubUserUrl);
  return /^https:\/\/github\.com\/[^/]+$/.test(n);
}

function syncNotesConnectionAccess() {
  const allowed = isGithubAccountConfigured();
  const input = document.getElementById('notes-connect-url');
  const addBtn = document.getElementById('btn-notes-connect-add');
  if (input) {
    input.disabled = !allowed;
    input.placeholder = allowed ? 'owner/repo or GitHub URL' : 'Set a Sync token first';
  }
  if (addBtn) addBtn.disabled = !allowed;
  if (!allowed && !notesGithubRepoInferredFromOrigin) {
    setResult('notes-connect-error', NOTES_CONNECT_NEEDS_ACCOUNT, true);
  } else if (allowed) {
    const el = document.getElementById('notes-connect-error');
    if (el?.textContent === NOTES_CONNECT_NEEDS_ACCOUNT) {
      setResult('notes-connect-error', '');
    }
  }
}

async function addNotesGithubRepo() {
  const input = document.getElementById('notes-connect-url');
  const btn = document.getElementById('btn-notes-connect-add');
  const url = normalizeNotesGithubRepoUrl(input?.value || '');
  if (!isGithubAccountConfigured()) {
    setResult('notes-connect-error', NOTES_CONNECT_NEEDS_ACCOUNT, true);
    return;
  }
  if (notesGithubRepoInferredFromOrigin) {
    setResult('notes-connect-error', 'Inferred from git origin (read-only).', true);
    return;
  }
  if (!url) {
    setResult('notes-connect-error', 'Enter a GitHub repository URL.', true);
    return;
  }
  if (savedSnapshot.workbenchGithubRepoUrl) {
    setResult('notes-connect-error', 'Delete the current repository before adding another.', true);
    return;
  }
  btn.disabled = true;
  setResult('notes-connect-error', '');
  try {
    await saveNotesGithubRepoUrl(url);
    if (input) input.value = '';
    setResult('notes-connect-error', 'Added.');
  } catch (e) {
    setResult('notes-connect-error', e.message || String(e), true);
  } finally {
    btn.disabled = false;
  }
}

async function deleteNotesGithubRepo() {
  if (notesGithubRepoInferredFromOrigin) {
    setResult('notes-connect-error', 'Inferred from git origin (read-only).', true);
    return;
  }
  const btn = document.getElementById('btn-notes-connect-delete');
  if (btn) btn.disabled = true;
  setResult('notes-connect-error', '');
  try {
    await saveNotesGithubRepoUrl('');
    setResult('notes-connect-error', 'Removed. You can add a repository again.');
  } catch (e) {
    setResult('notes-connect-error', e.message || String(e), true);
    renderNotesConnection(savedSnapshot.workbenchGithubRepoUrl);
  }
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

// ── Helpers ────────────────────────────────────────────────────────────────

function cursorIdeServerUrl() {
  return `http://127.0.0.1:${mcpPort}/mcp/cursor_ide`;
}

function formatCursorIdeServerBlock(handle) {
  return JSON.stringify(
    {
      url: cursorIdeServerUrl(),
      headers: { Authorization: `Bearer ${handle}` },
    },
    null,
    2,
  );
}

function setMcpServerBlock(text) {
  const el = document.getElementById('settings-mcp-server-block');
  if (el) el.value = text;
}

function clearMcpServerBlock() {
  setMcpServerBlock('');
}

async function copyServerBlock(text) {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    // Tauri webview often denies Clipboard API; keep the ticket in the field.
  }
  return false;
}

async function issueOrRotateCursorIdeBlock(cmd, copiedMessage, failLabel) {
  setResult('settings-result-mcp', '');
  try {
    const resp = await api.invoke(cmd);
    const issued = resp?.handle;
    if (!issued) throw new Error('Ticket command failed');
    const text = formatCursorIdeServerBlock(issued);
    setMcpServerBlock(text);
    const copied = await copyServerBlock(text);
    const doneLabel = failLabel === 'Rotate' ? 'Rotated' : 'Generated';
    setResult(
      'settings-result-mcp',
      copied
        ? copiedMessage
        : `${doneLabel} cursor_ide server block. Clipboard copy was blocked — copy the block from the field.`,
    );
  } catch (e) {
    clearMcpServerBlock();
    setResult('settings-result-mcp', `${failLabel} failed: ${e.message || String(e)}`, true);
  }
}

async function generateCursorIdeServerBlock() {
  const btn = document.getElementById('btn-settings-mcp-generate');
  if (btn) btn.disabled = true;
  try {
    await issueOrRotateCursorIdeBlock(
      'issue_cursor_ide_ticket',
      'Generated and copied cursor_ide server block.',
      'Generate',
    );
  } finally {
    if (btn) btn.disabled = false;
  }
}

async function rotateCursorIdeTicket() {
  const btn = document.getElementById('btn-settings-mcp-rotate');
  if (btn) btn.disabled = true;
  try {
    await issueOrRotateCursorIdeBlock(
      'rotate_cursor_ide_ticket',
      'Rotated and copied cursor_ide server block.',
      'Rotate',
    );
  } finally {
    if (btn) btn.disabled = false;
  }
}

async function revokeMcpSlotTicket() {
  const btn = document.getElementById('btn-settings-mcp-revoke');
  const slot = document.getElementById('settings-mcp-revoke-slot')?.value;
  if (btn) btn.disabled = true;
  setResult('settings-result-mcp', '');
  try {
    await api.invoke('revoke_mcp_slot_ticket', { slot });
    setResult('settings-result-mcp', `Revoked ${slot} ticket.`);
  } catch (e) {
    setResult('settings-result-mcp', `Revoke failed: ${e.message || String(e)}`, true);
  } finally {
    if (btn) btn.disabled = false;
  }
}

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
  workbenchGithubRepoUrl: '',
  hasGithubToken: false,
};

/** Origin 推断锁：推断成功后禁止手改 github_user_url。 */
let githubUserUrlInferredFromOrigin = '';
let notesGithubRepoInferredFromOrigin = '';

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
  return id === 'host' ? id : DEFAULT_ENGINE_CATEGORY;
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
  const hasKey = categoryId === 'host' && engineKeyHints.has_host_key;
  return hasKey
    ? 'API key configured. Enter a new key to replace it.'
    : 'No API key configured.';
}

function resolveEnginePreset(categoryId) {
  return getEnginePreset(categoryId) || getEnginePreset(DEFAULT_ENGINE_CATEGORY);
}

function fillReadonlyPresetFields(categoryId) {
  const preset = resolveEnginePreset(categoryId);
  const platformInput = document.getElementById('settings-llm-platform');
  const baseUrlInput = document.getElementById('settings-llm-base-url');
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
}

/**
 * Fill Assistant/Engine panel from config (category, readonly preset, model, credential hint).
 * @param {Record<string, unknown>} cfg
 */
function loadAssistantEnginePanel(cfg) {
  ensureEngineCategoryOptions();
  const categoryId = normalizeEngineCategory(cfg?.assistant_engine);
  const llm = cfg?.llm ?? {};

  engineKeyHints.has_host_key = Boolean(cfg?.has_host_key);
  const engineSelect = document.getElementById('settings-llm-engine');
  const modelInput = document.getElementById('settings-llm-model');
  const keyHint = document.getElementById('settings-llm-key-hint');
  const apiKeyInput = document.getElementById('settings-llm-api-key');

  activeEngineCategory = categoryId;
  // The facade exposes only the Host/GLM model.
  engineModelByCategory.host = undefined;
  engineModelByCategory[categoryId] =
    typeof llm.model === 'string' ? llm.model : '';

  if (engineSelect) engineSelect.value = categoryId;
  fillReadonlyPresetFields(categoryId);
  if (modelInput) {
    modelInput.value = engineModelByCategory[categoryId] ?? '';
    modelInput.readOnly = false;
    modelInput.disabled = false;
  }
  if (apiKeyInput) apiKeyInput.value = '';
  if (keyHint) keyHint.textContent = credentialHintForCategory(categoryId);
}

/**
 * Rebind panel to the Host/GLM model + readonly preset fields.
 * @param {string} categoryId
 * @param {{ clearCredential?: boolean }} [opts]
 */
async function applyEngineCategorySelection(
  categoryId,
  { clearCredential = true } = {},
) {
  const id = normalizeEngineCategory(categoryId);
  const prev = activeEngineCategory;
  const engineSelect = document.getElementById('settings-llm-engine');
  const modelInput = document.getElementById('settings-llm-model');
  const keyHint = document.getElementById('settings-llm-key-hint');
  const apiKeyInput = document.getElementById('settings-llm-api-key');

  if (modelInput) {
    engineModelByCategory[prev] = modelInput.value;
  }
  activeEngineCategory = id;

  if (engineSelect) engineSelect.value = id;
  fillReadonlyPresetFields(id);
  if (clearCredential && apiKeyInput) apiKeyInput.value = '';
  if (keyHint) keyHint.textContent = credentialHintForCategory(id);

  if (modelInput && activeEngineCategory === id) {
    modelInput.value = engineModelByCategory[id] ?? '';
    modelInput.readOnly = false;
    modelInput.disabled = false;
  }
}

async function saveAssistantEnginePanel() {
  const btn = document.getElementById('btn-settings-save-llm');
  const categoryId = normalizeEngineCategory(
    document.getElementById('settings-llm-engine')?.value,
  );
  const model = document.getElementById('settings-llm-model')?.value.trim() ?? '';
  const apiKey = document.getElementById('settings-llm-api-key')?.value.trim() ?? '';

  const payload = {
    assistant_engine: 'host',
    llm: { model },
  };
  if (apiKey) {
    payload.api_key_host = apiKey;
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
    applyNotesGithubRepoFromInferResponse({});
    return;
  }

  let resp;
  try {
    resp = await api.inferGithubUserUrl(root);
  } catch (e) {
    clearGithubUserUrlInferredLock();
    applyNotesGithubRepoFromInferResponse({});
    setResult(
      'settings-result-github',
      `Could not infer GitHub profile: ${e.message || String(e)}. If you just updated the app, fully restart and try again.`,
      true,
    );
    return;
  }

  applyNotesGithubRepoFromInferResponse(resp);
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
    applyNotesGithubRepoFromInferResponse({});
    return { ok: true };
  }

  let resp;
  try {
    resp = await api.inferGithubUserUrl(root);
  } catch (e) {
    clearGithubUserUrlInferredLock();
    applyNotesGithubRepoFromInferResponse({});
    return { ok: false, error: e.message || String(e) };
  }

  applyNotesGithubRepoFromInferResponse(resp);
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
    const githubUserInput = document.getElementById('settings-github-user-url');
    const corpusInput = document.getElementById('sediment-kb-corpus-path');
    const wbRoot = cfg?.workbench_knowledge_root ?? '';
    const corpusRoot = cfg?.knowledge_corpus_root ?? '';
    const ghUrl = cfg?.github_user_url ?? '';
    if (wbRoot) {
      archiveInput.placeholder = wbRoot;
      archiveInput.value = wbRoot;
    }
    if (corpusInput) {
      corpusInput.value = corpusRoot || state.ui.knowledgeCorpusRoot || '';
      if (corpusRoot) {
        corpusInput.placeholder = corpusRoot;
        state.ui.knowledgeCorpusRoot = corpusRoot;
      }
    }
    clearGithubUserUrlInferredLock();
    if (githubUserInput) {
      githubUserInput.value = ghUrl;
    }
    setGithubUserUrl(ghUrl);
    savedSnapshot.workbenchKnowledgeRoot = wbRoot;
    savedSnapshot.githubUserUrl = ghUrl;
    savedSnapshot.workbenchGithubRepoUrl = cfg?.workbench_github_repo_url ?? '';
    savedSnapshot.hasGithubToken = Boolean(cfg?.has_github_token);
    renderNotesConnection(savedSnapshot.workbenchGithubRepoUrl);
    setResult('notes-connect-error', '');

    const hintEl = document.getElementById('settings-token-hint');
    hintEl.textContent = cfg?.has_github_token
      ? 'Sync token configured. Enter a new token to replace it.'
      : 'No Sync token configured.';

    loadAssistantEnginePanel(cfg ?? {});
    if (typeof cfg?.mcp_port === 'number' && cfg.mcp_port > 0) {
      mcpPort = cfg.mcp_port;
    }

    await syncGithubUserUrlLockFromWorkbenchRoot();
    syncKbHidePatternInput();
    syncNotesConnectionAccess();
  } catch {
    document.getElementById('settings-token-hint').textContent =
      'Could not load settings; you can type and save.';
    document.getElementById('settings-llm-key-hint').textContent =
      'Could not load settings; you can type and save.';
    loadAssistantEnginePanel({});
    clearGithubUserUrlInferredLock();
    syncKbHidePatternInput();
    savedSnapshot.githubUserUrl = '';
    savedSnapshot.workbenchGithubRepoUrl = '';
    savedSnapshot.hasGithubToken = false;
    renderNotesConnection('');
    syncNotesConnectionAccess();
  }
}

// ── Open / close ───────────────────────────────────────────────────────────

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

// ── Workbench root blur: infer GitHub profile ─────────────────────────────────

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

// ── Save: 目录配置 ──────────────────────────────────────────────────────────

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
  if (notesGithubRepoInferredFromOrigin && isGithubAccountConfigured()) {
    payload.workbench_github_repo_url = notesGithubRepoInferredFromOrigin;
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
  } else if (githubUserUrlInferredFromOrigin) {
    payload.github_user_url = githubUserUrlInferredFromOrigin;
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
