import { state, getEntryId, loadDiffStatus } from '../state.js'
import { getGithubUserUrl, workbenchGithubBlobBase } from '../constants.js'
import { getActivePath } from '../corpus-path.js'
import { escHtml, filenameFromPath, slugToTitle, resetEditAreaScroll } from '../utils.js'
import * as api from '../api.js'
import { updateTitlesInDOM, updateDiffInDOM } from './cards.js'
import { renderLinksBar } from './links-bar.js'
import { renderTagsBar } from './tags-bar.js'
import { renderComments } from './comments.js'
import { openDeleteDialog } from './modals/delete-dialog.js'
import { applyHighlights, initHighlightUI } from './highlights.js'
import { initMermaid, renderMermaidBlocks } from '../mermaid-render.js'
import { openKbDoc, saveKbDoc } from './kb-viewer.js'
export { openKbDoc }
import { mountKnowledgeSearch, triggerKnowledgeSearch } from './knowledge-search.js'

// ── Pending-commit badge ─────────────────────────────────────────────────────

export function showPendingBadge() {
  const btn = document.getElementById('btn-panel-commit');
  if (btn) btn.style.display = '';
}

export function hidePendingBadge() {
  const btn = document.getElementById('btn-panel-commit');
  if (btn) btn.style.display = 'none';
}

function closeCommitDialog() {
  document.getElementById('md-commit-dialog').classList.remove('open');
}

export async function openCommitDialog() {
  const dialog = document.getElementById('md-commit-dialog');
  const fileList = document.getElementById('md-commit-file-list');
  const msgInput = document.getElementById('md-commit-dialog-msg');
  const resultEl = document.getElementById('md-commit-dialog-result');
  const okBtn = document.getElementById('md-btn-commit-ok');

  msgInput.value = '';
  resultEl.textContent = '';
  resultEl.style.color = '';
  fileList.innerHTML = '<div style="font-size:12px;color:#8c959f;">Loading…</div>';
  okBtn.disabled = false;
  dialog.classList.add('open');

  try {
    const data = await api.fetchDiffStatus();
    if (!data) throw new Error('Could not get status');
    if (data.error) throw new Error(data.error);

    if (!data.total && !data.ahead) {
      fileList.innerHTML = '<div style="font-size:13px;color:#8c959f;padding:4px 0;">No changes to commit</div>';
      okBtn.disabled = true;
      return;
    }

    const GROUPS = [
      { key: 'new',        label: 'New' },
      { key: 'modified',   label: 'Modified' },
      { key: 'deleted',    label: 'Deleted' },
      { key: 'renamed',    label: 'Renamed' },
      { key: 'conflicted', label: 'Conflict' },
    ];
    let html = '<div style="display:flex;flex-direction:column;gap:10px;">';
    for (const { key, label } of GROUPS) {
      if (data[key]?.length) {
        html += `<div class="commit-file-group">
          <div class="commit-file-group-title">${label}（${data[key].length}）</div>
          ${data[key].map(f => `<div class="commit-file-item ${key}">
            <span>${escHtml(f)}</span>
            <button class="kb-revert-btn" data-path="${escHtml(f)}" data-type="${key}">Revert</button>
          </div>`).join('')}
        </div>`;
      }
    }
    html += '</div>';
    fileList.innerHTML = html;
  } catch (e) {
    fileList.innerHTML = `<div style="font-size:12px;color:#cf222e;">Failed to get status: ${escHtml(e.message)}</div>`;
  }
}

// ── resolveRelativeLink ────────────────────────────────────────────────────

function resolveRelativeLink(href, layer, commonPath) {
  const ghBase = workbenchGithubBlobBase(getGithubUserUrl(), state.ui.workbenchKnowledgeRoot);
  if (!ghBase) return null;
  try {
    const base = `http://x/${layer}/${commonPath}`;
    const resolved = new URL(href, base);
    const repoPath = resolved.pathname.slice(1);
    return `${ghBase}/${repoPath}`;
  } catch {
    return null;
  }
}

// ── postProcessLinks ───────────────────────────────────────────────────────

function postProcessLinks(container, layer, commonPath) {
  container.querySelectorAll('a[href]').forEach(a => {
    const href = a.getAttribute('href');
    if (!href || href.startsWith('#')) return;
    if (href.startsWith('http://') || href.startsWith('https://')) {
      a.target = '_blank';
      a.rel = 'noopener noreferrer';
      return;
    }
    const ghUrl = resolveRelativeLink(href, layer, commonPath);
    if (ghUrl) {
      a.href = ghUrl;
      a.target = '_blank';
      a.rel = 'noopener noreferrer';
    }
  });
}

const _corpusBlobUrls = new Set();

function revokeCorpusBlobUrls() {
  for (const url of _corpusBlobUrls) {
    URL.revokeObjectURL(url);
  }
  _corpusBlobUrls.clear();
}

function isExternalOrSpecialImgSrc(src) {
  return /^(https?:|data:|blob:|\/)/i.test(src);
}

async function postProcessImages(container, layer, commonPath) {
  const imgs = [...container.querySelectorAll('img[src]')];
  await Promise.all(
    imgs.map(async (img) => {
      const href = img.getAttribute('src');
      if (!href || href.startsWith('#') || isExternalOrSpecialImgSrc(href)) return;
      try {
        const blobUrl = await api.fetchCorpusAssetAsBlobUrl(layer, commonPath, href);
        _corpusBlobUrls.add(blobUrl);
        img.src = blobUrl;
      } catch {
        img.alt = img.alt || href;
      }
    }),
  );
}

// ── renderDocBody ──────────────────────────────────────────────────────────

export async function renderDocBody(text, layer, commonPath) {
  const body = document.getElementById('md-body');
  if (typeof marked !== 'undefined') {
    body.innerHTML = marked.parse(text);
  } else {
    body.innerHTML = `<pre style="white-space:pre-wrap;font-size:13px">${escHtml(text)}</pre>`;
  }
  postProcessLinks(body, layer, commonPath);
  document.getElementById('btn-edit').style.display = '';
  renderLinksBar(state.viewer.entry);
  renderTagsBar(state.viewer.entry);
  revokeCorpusBlobUrls();
  await postProcessImages(body, layer, commonPath);
  await renderMermaidBlocks(body);
  renderComments(state.viewer.annotation, layer, state.viewer.entry);
  const zone = document.createElement('div');
  zone.className = 'md-body-delete-zone';
  const delBtn = document.createElement('button');
  delBtn.id = 'btn-delete';
  delBtn.textContent = '🗑 Delete this entry';
  delBtn.title = 'Deletes all linked files (raw / distilled / trace / digest / diagnose)';
  delBtn.addEventListener('click', () => openDeleteDialog());
  zone.appendChild(delBtn);
  body.appendChild(zone);
}

// ── Language helpers ───────────────────────────────────────────────────────

function updateLangBar(entry) {
  const bar = document.getElementById('md-lang-bar');
  const hasZh = !!entry.translations?.zh;
  bar.style.display = hasZh ? 'flex' : 'none';
  if (hasZh) {
    document.getElementById('btn-lang-en').classList.toggle('active', state.viewer.lang !== 'zh');
    document.getElementById('btn-lang-zh').classList.toggle('active', state.viewer.lang === 'zh');
  }
}

function updateHeaderUrls(entry, layer, activePath) {
  const ghBase = workbenchGithubBlobBase(getGithubUserUrl(), state.ui.workbenchKnowledgeRoot);
  const githubUrl = ghBase ? `${ghBase}/${layer}/${activePath}` : '';
  const ghLink = document.getElementById('md-github-link');
  const copyHttp = document.getElementById('btn-copy-http');
  if (githubUrl) {
    ghLink.href = githubUrl;
    ghLink.style.display = '';
    copyHttp.dataset.url = githubUrl;
    copyHttp.dataset.tip = githubUrl;
    copyHttp.style.display = '';
  } else {
    ghLink.removeAttribute('href');
    ghLink.style.display = 'none';
    copyHttp.style.display = 'none';
  }
  const relPath = `${layer}/${activePath}`;
  const fullPath = state.ui.workbenchKnowledgeRoot ? `${state.ui.workbenchKnowledgeRoot}/${relPath}` : relPath;
  document.getElementById('btn-copy-path').dataset.tip = fullPath;

  const topicDir = entry.common_path.split('/')[0];
  const kbUrl = state.index.topicRepos[topicDir];
  const kbBtn = document.getElementById('btn-goto-kb');
  if (kbUrl) {
    kbBtn.style.display = '';
    kbBtn.onclick = () => window.open(kbUrl, '_blank', 'noopener,noreferrer');
  } else {
    kbBtn.style.display = 'none';
  }
}

// ── openDoc ────────────────────────────────────────────────────────────────

export async function openDoc(entry, layer = 'raw') {
  state.viewer.entry = entry;
  state.viewer.layer = layer;
  state.viewer.annotation = {};
  state.viewer.lang = entry.translations?.zh ? 'zh' : null;
  exitEditMode(false);
  closeCommitDialog();

  const modal = document.getElementById('md-modal');
  const body = document.getElementById('md-body');
  document.getElementById('md-panel-title').textContent = filenameFromPath(entry.common_path).replace(/\.md$/, '');
  const activePath = getActivePath(entry, state.viewer.lang, layer);
  updateHeaderUrls(entry, layer, activePath);
  updateLangBar(entry);
  document.getElementById('md-file-size').textContent = '';
  body.innerHTML = '<div style="color:#8c959f;padding:20px;font-size:13px;">Loading…</div>';
  modal.style.display = 'flex';
  document.body.style.overflow = 'hidden';

  const [mdResult, annResult] = await Promise.allSettled([
    api.fetchFileContent(layer, activePath),
    api.fetchAnnotation(entry.common_path).catch(() => ({}))
  ]);

  if (annResult.status === 'fulfilled') state.viewer.annotation = annResult.value || {};

  if (mdResult.status === 'rejected') {
    body.innerHTML = `<div style="color:#7d4e00;padding:20px">Could not load file: ${escHtml(mdResult.reason.message)}</div>`;
    document.getElementById('btn-edit').style.display = 'none';
    return;
  }

  const text = mdResult.value;
  state.viewer.rawText = text;
  const bytes = new Blob([text]).size;
  document.getElementById('md-file-size').textContent = bytes < 1024
    ? `${bytes} B`
    : bytes < 1024 * 1024
      ? `${(bytes / 1024).toFixed(1)} KB`
      : `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  await renderDocBody(text, layer, activePath);
  applyHighlights(state.viewer.annotation, layer);
  const hasDiff = state.index.diffStatus.get(`${layer}/${entry.common_path}`);
  if (hasDiff) showPendingBadge(); else hidePendingBadge();

  mountKnowledgeSearch(document.getElementById('knowledge-panel'));
  triggerKnowledgeSearch(entry);
}

// ── switchLang ─────────────────────────────────────────────────────────────

export async function switchLang(lang) {
  if (!state.viewer.entry) return;
  const entry = state.viewer.entry;
  const layer = state.viewer.layer;

  // save current scroll position
  const body = document.getElementById('md-body');
  const cacheKey = `${getEntryId(entry)}:${layer}`;
  state.viewer.scrollCache[cacheKey] = body.scrollTop;

  state.viewer.lang = lang;
  const activePath = getActivePath(entry, lang, layer);
  updateHeaderUrls(entry, layer, activePath);
  updateLangBar(entry);

  body.innerHTML = '<div style="color:#8c959f;padding:20px;font-size:13px;">Loading…</div>';

  try {
    const text = await api.fetchFileContent(layer, activePath);
    state.viewer.rawText = text;
    const bytes = new Blob([text]).size;
    document.getElementById('md-file-size').textContent = bytes < 1024
      ? `${bytes} B`
      : bytes < 1024 * 1024
        ? `${(bytes / 1024).toFixed(1)} KB`
        : `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    await renderDocBody(text, layer, activePath);
    applyHighlights(state.viewer.annotation, layer);
    // restore scroll position
    const cacheKey = `${getEntryId(entry)}:${layer}`;
    const saved = state.viewer.scrollCache[cacheKey];
    if (saved != null) requestAnimationFrame(() => { body.scrollTop = saved; });
  } catch (e) {
    body.innerHTML = `<div style="color:#7d4e00;padding:20px">Could not load file: ${escHtml(e.message)}</div>`;
  }
}

// ── Edit mode ──────────────────────────────────────────────────────────────

export function enterEditMode() {
  const editArea = document.getElementById('md-edit-area');
  const body = document.getElementById('md-body');
  editArea.value = state.viewer.rawText;
  body.style.display = 'none';
  editArea.style.display = '';
  resetEditAreaScroll(editArea, { focus: true });
  document.getElementById('btn-edit').style.display = 'none';
  document.getElementById('btn-add-comment').style.display = 'none';
  hidePendingBadge();
  document.getElementById('btn-save').style.display = '';
  document.getElementById('btn-cancel-edit').style.display = '';
  document.getElementById('md-github-link').style.display = 'none';
  closeCommitDialog();
}

export function exitEditMode(rerender = true) {
  const editArea = document.getElementById('md-edit-area');
  const body = document.getElementById('md-body');
  editArea.style.display = 'none';
  body.style.display = '';
  document.getElementById('btn-edit').style.display = '';
  document.getElementById('btn-add-comment').style.display = '';
  document.getElementById('btn-save').style.display = 'none';
  document.getElementById('btn-cancel-edit').style.display = 'none';
  const _layer = state.viewer.layer || 'raw';
  const _cp = state.viewer.entry?.common_path;
  const _hasDiff = _cp ? state.index.diffStatus.get(`${_layer}/${_cp}`) : false;
  if (_hasDiff) showPendingBadge(); else hidePendingBadge();
  document.getElementById('md-github-link').style.display = '';
  if (rerender && state.viewer.entry) {
    const activePath = getActivePath(state.viewer.entry, state.viewer.lang, state.viewer.layer);
    void renderDocBody(state.viewer.rawText, state.viewer.layer, activePath);
  }
}

initMermaid();

export async function saveDoc() {
  if (state.viewer.isKb) {
    await saveKbDoc();
    return;
  }
  if (!state.viewer.entry) return;
  const editArea = document.getElementById('md-edit-area');
  const newContent = editArea.value;
  const btnSave = document.getElementById('btn-save');
  btnSave.disabled = true;
  btnSave.textContent = 'Saving…';

  try {
    const activePath = getActivePath(state.viewer.entry, state.viewer.lang, state.viewer.layer);
    const data = await api.saveFile(state.viewer.layer, activePath, newContent);
    if (data.error) throw new Error(data.error);

    state.viewer.rawText = newContent;
    const date = state.viewer.entry.created_at.slice(0, 8);
    const h1Match = newContent.match(/^#\s+(.+)/m);
    const newTitle = h1Match ? h1Match[1].trim() : slugToTitle(filenameFromPath(state.viewer.entry.common_path));
    const entryId = getEntryId(state.viewer.entry);
    if (!state.index.titleCache.has(date)) state.index.titleCache.set(date, new Map());
    state.index.titleCache.get(date).set(entryId, newTitle);
    const card = entryId ? document.querySelector(`.doc-card[data-id="${entryId}"]`) : null;
    if (card) {
      const titleEl = card.querySelector('.doc-title-btn');
      if (titleEl) { titleEl.classList.remove('loading'); titleEl.textContent = newTitle; }
    } else {
      updateTitlesInDOM(date);
    }
    exitEditMode(true);
    await loadDiffStatus();
    updateDiffInDOM();
    showPendingBadge();
  } catch (e) {
    alert(`Save failed: ${e.message}`);
  } finally {
    btnSave.disabled = false;
    btnSave.textContent = '💾 Save';
  }
}

// bridge: exitEditMode still called by openDoc in this file (no window needed internally)
// index.html Step 4.2 removes window.exitEditMode assignment

// ── Commit bar ─────────────────────────────────────────────────────────────

export function showCommitBar() {
  document.getElementById('md-commit-msg').value = '';
  document.getElementById('md-commit-result').textContent = '';
  document.getElementById('btn-panel-commit').style.display = 'none';
  document.getElementById('md-commit-bar').style.display = 'flex';
  document.getElementById('md-commit-msg').focus();
}

export function hideCommitBar() {
  document.getElementById('md-commit-bar').style.display = 'none';
}

export async function commitCurrentFile() {
  if (state.viewer.isKb) return; // KB uses its own commit dialog
  if (!state.viewer.entry) return;
  const msg = document.getElementById('md-commit-msg').value.trim() || 'update: edit via viewer';
  const resultEl = document.getElementById('md-commit-result');
  const btn = document.getElementById('btn-commit-file');
  btn.disabled = true;
  btn.textContent = 'Committing…';
  resultEl.style.color = '#57606a';
  resultEl.textContent = '';

  try {
    const activePath = getActivePath(state.viewer.entry, state.viewer.lang, state.viewer.layer);
    const filePath = `${state.viewer.layer}/${activePath}`;
    const data = await api.commitFiles(msg, [filePath]);
    if (data.error) throw new Error(data.error + (data.stderr ? '\n' + data.stderr : ''));

    resultEl.style.color = '#1a7f37';
    resultEl.textContent = '✓ Pushed!';
    await loadDiffStatus();
    updateDiffInDOM();
    document.dispatchEvent(new CustomEvent('cta:reload'));
    setTimeout(hideCommitBar, 1500);
  } catch (e) {
    resultEl.style.color = '#cf222e';
    resultEl.textContent = `✗ ${e.message}`;
  } finally {
    btn.disabled = false;
    btn.textContent = '↑ Commit';
  }
}

// ── Close viewer ───────────────────────────────────────────────────────────

const CREATE_CHROME_HIDDEN_IDS = [
  'btn-edit',
  'btn-add-comment',
  'btn-save',
  'btn-cancel-edit',
  'btn-panel-commit',
  'md-github-link',
  'md-lang-bar',
  'md-file-size',
  'md-links-bar',
  'md-tags-bar',
  'knowledge-panel',
  'btn-copy-http',
  'btn-copy-path',
  'btn-goto-kb',
  'btn-open-iterm',
  'comment-float-nav',
  'md-commit-bar',
];

/** @type {Record<string, string>|null} */
let createChromePrevDisplay = null;

function applyCreateChrome() {
  const modal = document.getElementById('md-modal');
  modal.classList.add('is-create');
  createChromePrevDisplay = {};
  for (const id of CREATE_CHROME_HIDDEN_IDS) {
    const el = document.getElementById(id);
    if (!el) continue;
    createChromePrevDisplay[id] = el.style.display;
    el.style.display = 'none';
  }
  document.getElementById('md-panel-title').textContent = 'New note';
  document.getElementById('md-body').style.display = 'none';
  const editArea = document.getElementById('md-edit-area');
  editArea.style.display = '';
}

function clearCreateChrome() {
  const modal = document.getElementById('md-modal');
  modal.classList.remove('is-create');
  if (createChromePrevDisplay) {
    for (const [id, display] of Object.entries(createChromePrevDisplay)) {
      const el = document.getElementById(id);
      if (el) el.style.display = display;
    }
    createChromePrevDisplay = null;
  }
}

function dismissViewerModal() {
  clearCreateChrome();
  document.getElementById('md-modal').style.display = 'none';
  document.body.style.overflow = '';
  exitEditMode(false);
  closeCommitDialog();
}

/**
 * Enter create session on the shared viewer (body-only chrome).
 * Binds crash buffer at drafts/notes/<temp_id>. Does not pretend an index entry exists.
 * @param {{ temp_id: string }} opts
 */
export async function openCreateNote({ temp_id } = {}) {
  if (!temp_id) return;
  const modal = document.getElementById('md-modal');
  const prevDisplay = modal.style.display;
  try {
    let content = '';
    try {
      const draft = await api.getNoteDraft(temp_id);
      content = typeof draft?.content === 'string' ? draft.content : '';
    } catch {
      content = '';
    }
    await api.saveNoteDraft(temp_id, content);

    state.viewer.entry = null;
    state.viewer.layer = 'raw';
    state.viewer.lang = null;
    state.viewer.annotation = {};
    state.viewer.rawText = content;
    state.viewer.isKb = false;
    state.viewer.createSession = { tempId: temp_id, status: 'creating' };

    closeCommitDialog();
    applyCreateChrome();

    const editArea = document.getElementById('md-edit-area');
    editArea.value = content;
    resetEditAreaScroll(editArea, { focus: true });

    modal.style.display = 'flex';
    document.body.style.overflow = 'hidden';
  } catch (e) {
    state.viewer.createSession = null;
    clearCreateChrome();
    modal.style.display = prevDisplay;
    const msg = e instanceof Error ? e.message : String(e);
    alert(`Could not open new note: ${msg}`);
  }
}

async function finalizeCreateSession() {
  const session = state.viewer.createSession;
  if (!session || session.status === 'saving') return;

  const editArea = document.getElementById('md-edit-area');
  const trimmed = (editArea?.value ?? '').trim();

  if (!trimmed) {
    await api.clearNoteDraft(session.tempId);
    state.viewer.createSession = null;
    dismissViewerModal();
    return;
  }

  session.status = 'saving';
  try {
    await api.saveNoteDraft(session.tempId, trimmed);
    await api.archiveDocument({ body: trimmed, source_type: 'note' });
    await api.clearNoteDraft(session.tempId);
    state.viewer.createSession = null;
    dismissViewerModal();
    document.dispatchEvent(new CustomEvent('cta:reload'));
  } catch (e) {
    session.status = 'creating';
    alert(`Save failed: ${e.message}`);
  }
}

export async function closeModal() {
  if (state.viewer.createSession) {
    await finalizeCreateSession();
    return;
  }
  dismissViewerModal();
}

document.getElementById('md-close').addEventListener('click', () => { void closeModal(); });
document.getElementById('md-backdrop').addEventListener('click', () => { void closeModal(); });
document.addEventListener('keydown', e => {
  if (e.key === 'Escape') {
    // Don't close modal if comment dialog is open — let it handle ESC itself
    if (document.getElementById('comment-dialog').classList.contains('open')) return;
    if (document.getElementById('md-commit-dialog').classList.contains('open')) return;
    void closeModal();
  }
});

// ── Workbench Commit dialog ────────────────────────────────────────────────

document.getElementById('md-btn-commit-cancel').addEventListener('click', () => {
  closeCommitDialog();
});

document.getElementById('md-commit-dialog').addEventListener('click', e => {
  if (e.target === document.getElementById('md-commit-dialog')) closeCommitDialog();
});

document.getElementById('md-btn-commit-ok').addEventListener('click', async () => {
  const btn = document.getElementById('md-btn-commit-ok');
  const resultEl = document.getElementById('md-commit-dialog-result');
  const msg = document.getElementById('md-commit-dialog-msg').value.trim() || 'update: edit via viewer';
  btn.disabled = true;
  resultEl.textContent = 'Committing…';
  resultEl.style.color = '#8c959f';
  try {
    const data = await api.commitFiles(msg);
    if (data.error) throw new Error(data.error);
    resultEl.style.color = '#1a7f37';
    resultEl.textContent = '✓ Pushed!';
    await loadDiffStatus();
    updateDiffInDOM();
    hidePendingBadge();
    setTimeout(() => closeCommitDialog(), 1500);
  } catch (e) {
    resultEl.style.color = '#cf222e';
    resultEl.textContent = `✗ ${e.message}`;
    btn.disabled = false;
  }
});

document.getElementById('md-btn-revert-all').addEventListener('click', async () => {
  const btn = document.getElementById('md-btn-revert-all');
  if (!btn.classList.contains('confirm')) {
    btn.classList.add('confirm');
    btn.textContent = 'Revert all changes?';
    setTimeout(() => {
      btn.classList.remove('confirm');
      btn.textContent = 'Revert all changes';
    }, 3000);
    return;
  }
  btn.disabled = true;
  try {
    const data = await api.revertFile('', '');
    if (data.error) throw new Error(data.error);
    await loadDiffStatus();
    updateDiffInDOM();
    hidePendingBadge();
    closeCommitDialog();
  } catch (e) {
    document.getElementById('md-commit-dialog-result').textContent = `Revert failed: ${e.message}`;
    btn.disabled = false;
  } finally {
    btn.classList.remove('confirm');
    btn.textContent = 'Revert all changes';
  }
});

document.getElementById('md-commit-file-list').addEventListener('click', async e => {
  const revertBtn = e.target.closest?.('.kb-revert-btn');
  if (!revertBtn) return;
  const path = revertBtn.dataset.path;
  const type = revertBtn.dataset.type;
  revertBtn.disabled = true;
  try {
    const data = await api.revertFile(path, type);
    if (data.error) throw new Error(data.error);
    await openCommitDialog();
  } catch (e_) {
    revertBtn.disabled = false;
    document.getElementById('md-commit-dialog-result').textContent = `Revert failed: ${escHtml(e_.message)}`;
  }
});

// ── Copy buttons ───────────────────────────────────────────────────────────

document.getElementById('btn-copy-http').addEventListener('click', () => {
  const url = document.getElementById('btn-copy-http').dataset.url || '';
  if (!url) return;
  navigator.clipboard.writeText(url).then(() => {
    const btn = document.getElementById('btn-copy-http');
    btn.textContent = '✓';
    setTimeout(() => { btn.textContent = '🌐'; }, 1200);
  });
});

document.getElementById('btn-copy-path').addEventListener('click', () => {
  const activePath = getActivePath(state.viewer.entry, state.viewer.lang, state.viewer.layer);
  const relPath = `${state.viewer.layer}/${activePath}`;
  const fullPath = state.ui.workbenchKnowledgeRoot ? `${state.ui.workbenchKnowledgeRoot}/${relPath}` : relPath;
  navigator.clipboard.writeText(fullPath).then(() => {
    const btn = document.getElementById('btn-copy-path');
    btn.textContent = '✓';
    setTimeout(() => { btn.textContent = '📂'; }, 1200);
  });
});

document.getElementById('btn-lang-en').addEventListener('click', () => switchLang('en'));
document.getElementById('btn-lang-zh').addEventListener('click', () => switchLang('zh'));

initHighlightUI();

