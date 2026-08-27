import { state } from '../host/state.js'
import { escHtml, resetEditAreaScroll } from '../shared/utils.js'
import * as api from '../host/api.js'
import { renderKbComments, initKbComments, cleanupKbComments } from './corpus-comments.js'
import { renderKbLinksBar } from './corpus-links-bar.js'
import { renderMermaidBlocks } from '../shared/mermaid-render.js'
import { knowledgeDocKey } from '../doc-editor/identity.js'
import {
  applyCachedHighlights,
  initDocHighlightOverlay,
  cleanupDocHighlightOverlay,
} from '../doc-editor/highlights.js'
import { renderDocMarkdown, setDocEditMode } from '../doc-editor/view.js'

const KB_BODY_ID = 'kb-md-body'
const KB_HIGHLIGHT_BTN_ID = 'kb-highlight-add-btn'

/** @type {HTMLElement | null} */
let _kbHighlightRoot = null

function kbIdentityKey() {
  const { kbRepo, kbPath } = state.viewer
  if (!kbRepo || !kbPath) return ''
  return knowledgeDocKey(kbRepo, kbPath)
}

function kbBodyEl(root) {
  if (root) return root.querySelector('.kb-reader-body') || root.querySelector(`#${KB_BODY_ID}`)
  return document.getElementById(KB_BODY_ID)
}

function applyKbHighlights() {
  const body = kbBodyEl(_kbHighlightRoot)
  const identityKey = kbIdentityKey()
  if (!body || !identityKey) return
  void applyCachedHighlights({
    bodyEl: body,
    identityKey,
    excludeBarId: 'kb-md-comments-bar',
  })
}

function cleanupKbHighlightUI() {
  cleanupDocHighlightOverlay()
  _kbHighlightRoot = null
}

function initKbHighlightUI(container) {
  if (!container) {
    const legacyBody = document.getElementById(KB_BODY_ID)
    container = legacyBody?.closest('.kb-reader') ?? legacyBody?.parentElement
    if (!container) return
  }
  _kbHighlightRoot = container
  initDocHighlightOverlay({
    getBody: () => kbBodyEl(_kbHighlightRoot),
    getEditArea: () =>
      container.querySelector('.kb-reader-edit-area') ||
      document.getElementById('kb-md-edit-area'),
    getIdentityKey: () => kbIdentityKey(),
    excludeBarId: 'kb-md-comments-bar',
    buttonId: KB_HIGHLIGHT_BTN_ID,
  })
  applyKbHighlights()
}

// ── postProcessLinks (KB) ──────────────────────────────────────────────────
function postProcessKbLinks(container) {
  container.querySelectorAll('a[href]').forEach(a => {
    const href = a.getAttribute('href');
    if (!href || href.startsWith('#')) return;
    if (href.startsWith('http://') || href.startsWith('https://')) {
      a.target = '_blank';
      a.rel = 'noopener noreferrer';
    }
  });
}

async function renderKbMdBody(text, bodyEl) {
  const body = bodyEl ?? document.getElementById('kb-md-body');
  if (!body) return;
  renderDocMarkdown(body, text);
  postProcessKbLinks(body);
  await renderMermaidBlocks(body);
  renderKbComments(state.viewer.annotation);
  void applyKbHighlights();
  renderKbLinksBar(state.viewer.annotation);
}

let loadToken = 0;

function bindReaderListener(listeners, el, type, handler) {
  el.addEventListener(type, handler);
  listeners.push([el, type, handler]);
}

function formatFileSize(text) {
  const bytes = new Blob([text]).size;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function buildReaderTitle(repo, path) {
  const pathParts = (path || '').split('/');
  const fileName = pathParts.pop();
  const repoName = (repo || '').split('/').pop();
  return pathParts.length > 0 ? `${repoName}/.../${fileName}` : `${repoName}/${fileName}`;
}

function readerShellHtml() {
  return `
    <div class="kb-reader">
      <div class="kb-reader-header viewer-header">
        <span class="kb-reader-title viewer-panel-title"></span>
        <span class="kb-file-size" style="font-size:10px;color:#8c959f;flex-shrink:0;"></span>
        <a class="kb-github-link" href="#" target="_blank" style="font-size:12px;color:#0969da;text-decoration:none;flex-shrink:0;">GitHub ↗</a>
        <button type="button" class="md-header-btn kb-btn-open-iterm" style="display:none" title="Open repo folder in iTerm">⌨️ Terminal</button>
        <button type="button" class="md-header-btn kb-btn-copy-http" data-tip="">&#127760;</button>
        <button type="button" class="md-header-btn kb-btn-copy-path" data-tip="">&#128194;</button>
        <button type="button" class="md-header-btn kb-btn-edit">✏️ Edit</button>
        <button type="button" class="md-header-btn kb-btn-add-comment">💬 Comment</button>
        <button type="button" class="md-header-btn primary kb-btn-save" style="display:none">💾 Save</button>
        <button type="button" class="md-header-btn kb-btn-cancel-edit" style="display:none">Cancel</button>
        <button type="button" class="md-header-btn kb-btn-pending" style="display:none">● Pending commit</button>
        <button type="button" class="md-header-btn kb-btn-reindex" style="display:none">↺ Rebuild index</button>
      </div>
      <div id="kb-md-links-bar" class="kb-reader-links-bar" style="display:none;padding:8px 20px;border-bottom:1px solid #d0d7de;"></div>
      <div class="kb-reader-content-row viewer-content-row">
        <div class="kb-reader-body viewer-body"></div>
        <textarea class="kb-reader-edit-area viewer-edit-area" style="display:none" spellcheck="false"></textarea>
        <div class="kb-comment-float-nav"></div>
      </div>
    </div>
  `;
}

function wireReindexBtn(btn, repo) {
  btn.textContent = '↺ Rebuild index';
  btn.title = 'Rebuild search index for this library';
  btn.disabled = false;
  btn.style.display = '';
  btn.onclick = async () => {
    btn.disabled = true;
    btn.textContent = 'Rebuilding…';
    try {
      const res = await api.reindexKbRepo(repo);
      if (res.error) throw new Error(res.error);
      const poll = setInterval(async () => {
        try {
          const status = await api.getReindexStatus();
          if (status.status === 'done') {
            clearInterval(poll);
            btn.textContent = '✓ Rebuilt';
            btn.disabled = false;
            setTimeout(() => { btn.style.display = 'none'; }, 2000);
          } else if (status.status === 'error') {
            clearInterval(poll);
            btn.textContent = 'Rebuild failed';
            btn.disabled = false;
            btn.title = status.log || 'Unknown error';
          }
        } catch (_) {}
      }, 2000);
    } catch (e) {
      btn.textContent = 'Rebuild failed';
      btn.disabled = false;
      btn.title = e.message;
    }
  };
}

/**
 * @param {HTMLElement} container
 * @param {{ repo: string, path: string, url?: string }} opts
 * @returns {Promise<{ unmount: () => void }>}
 */
export async function mountKbReader(container, { repo, path, url }) {
  if (container._kbUnmount) {
    container._kbUnmount();
  }

  loadToken += 1;
  const token = loadToken;

  state.viewer.entry = null;
  state.viewer.isKb = true;
  state.viewer.kbRepo = repo;
  state.viewer.kbPath = path;
  state.viewer.layer = 'raw';
  state.viewer.rawText = '';
  state.viewer.annotation = {};
  state.viewer.lang = null;

  container.innerHTML = readerShellHtml();

  const ui = {
    title: container.querySelector('.kb-reader-title'),
    fileSize: container.querySelector('.kb-file-size'),
    githubLink: container.querySelector('.kb-github-link'),
    itermBtn: container.querySelector('.kb-btn-open-iterm'),
    btnCopyHttp: container.querySelector('.kb-btn-copy-http'),
    btnCopyPath: container.querySelector('.kb-btn-copy-path'),
    btnEdit: container.querySelector('.kb-btn-edit'),
    btnSave: container.querySelector('.kb-btn-save'),
    btnCancelEdit: container.querySelector('.kb-btn-cancel-edit'),
    btnAddComment: container.querySelector('.kb-btn-add-comment'),
    btnPending: container.querySelector('.kb-btn-pending'),
    btnReindex: container.querySelector('.kb-btn-reindex'),
    body: container.querySelector('.kb-reader-body'),
    editArea: container.querySelector('.kb-reader-edit-area'),
  };

  /** @type {Array<[HTMLElement, string, (...args: any[]) => void]>} */
  const listeners = [];
  let pendingMsg = '';

  ui.title.textContent = buildReaderTitle(repo, path);
  ui.githubLink.href = url || '#';
  ui.btnCopyHttp.dataset.url = url || '';
  ui.btnCopyHttp.dataset.tip = url || '';
  const localPath = state.ui.knowledgeCorpusRoot
    ? `${state.ui.knowledgeCorpusRoot}/${(repo || '').split('/').pop()}/${path}`
    : `${(repo || '').split('/').pop()}/${path}`;
  ui.btnCopyPath.dataset.path = localPath;
  ui.btnCopyPath.dataset.tip = localPath;
  ui.fileSize.textContent = '';
  ui.itermBtn.style.display = '';
  wireReindexBtn(ui.btnReindex, repo);

  function showPendingBadge(msg) {
    pendingMsg = msg;
    ui.btnPending.style.display = '';
  }

  function hidePendingBadge() {
    pendingMsg = '';
    ui.btnPending.style.display = 'none';
  }

  function enterEditMode() {
    setDocEditMode({
      bodyEl: ui.body,
      editAreaEl: ui.editArea,
      text: state.viewer.rawText,
      editing: true,
    });
    ui.btnSave.style.display = '';
    ui.btnCancelEdit.style.display = '';
    ui.btnEdit.style.display = 'none';
    resetEditAreaScroll(ui.editArea, { focus: true });
  }

  function exitEditMode() {
    setDocEditMode({ bodyEl: ui.body, editAreaEl: ui.editArea, editing: false });
    ui.btnSave.style.display = 'none';
    ui.btnCancelEdit.style.display = 'none';
    ui.btnEdit.style.display = '';
  }

  async function saveDoc() {
    const newContent = ui.editArea.value;
    ui.btnSave.disabled = true;
    ui.btnSave.textContent = 'Saving…';
    try {
      const originalContent = state.viewer.rawText;
      const data = await api.saveKbFile(state.viewer.kbRepo, state.viewer.kbPath, newContent);
      if (data.error) throw new Error(data.error);
      state.viewer.rawText = newContent;
      exitEditMode();
      if (typeof marked !== 'undefined') {
        await renderKbMdBody(newContent, ui.body);
      } else {
        ui.body.innerHTML = `<pre style="white-space:pre-wrap;font-size:13px">${escHtml(newContent)}</pre>`;
      }
      ui.btnEdit.style.display = '';
      if (newContent !== originalContent) {
        showPendingBadge('update: edit via viewer');
        wireReindexBtn(ui.btnReindex, state.viewer.kbRepo);
      }
    } catch (e) {
      alert(`Save failed: ${e.message}`);
    } finally {
      ui.btnSave.disabled = false;
      ui.btnSave.textContent = '💾 Save';
    }
  }

  function onDirty(e) {
    showPendingBadge(e.detail?.msg || 'chore: update via viewer');
  }

  function unmount() {
    if (token !== loadToken) return;
    loadToken += 1;
    exitEditMode();
    cleanupKbComments();
    cleanupKbHighlightUI();
    for (const [el, type, handler] of listeners) {
      el.removeEventListener(type, handler);
    }
    document.removeEventListener('kb:dirty', onDirty);
    container.innerHTML = '';
    delete container._kbUnmount;
    state.viewer.isKb = false;
    state.viewer.kbRepo = null;
    state.viewer.kbPath = null;
    state.viewer.annotation = {};
    hidePendingBadge();
  }

  container._kbUnmount = unmount;

  bindReaderListener(listeners, ui.btnEdit, 'click', enterEditMode);
  bindReaderListener(listeners, ui.btnSave, 'click', () => { void saveDoc(); });
  bindReaderListener(listeners, ui.btnCancelEdit, 'click', exitEditMode);
  bindReaderListener(listeners, ui.btnPending, 'click', openKbCommitDialog);
  bindReaderListener(listeners, ui.btnCopyHttp, 'click', (e) => {
    const btn = e.currentTarget;
    const copyUrl = btn.dataset.url || '';
    if (!copyUrl) return;
    navigator.clipboard.writeText(copyUrl).then(() => {
      const orig = btn.textContent;
      btn.textContent = '✓';
      setTimeout(() => { btn.textContent = orig; }, 1200);
    }).catch(() => {});
  });
  bindReaderListener(listeners, ui.btnCopyPath, 'click', (e) => {
    const btn = e.currentTarget;
    const copyPath = btn.dataset.path || '';
    if (!copyPath) return;
    navigator.clipboard.writeText(copyPath).then(() => {
      const orig = btn.textContent;
      btn.textContent = '✓';
      setTimeout(() => { btn.textContent = orig; }, 1200);
    }).catch(() => {});
  });
  bindReaderListener(listeners, ui.itermBtn, 'click', async () => {
    ui.itermBtn.disabled = true;
    try {
      const res = await api.openItermAt(repo);
      if (res.error) alert(`Failed to open terminal: ${res.error}`);
    } catch (e) {
      alert(`Failed to open terminal: ${e.message}`);
    } finally {
      ui.itermBtn.disabled = false;
    }
  });

  document.addEventListener('kb:dirty', onDirty);

  ui.body.innerHTML = '<div style="color:#8c959f;padding:20px;font-size:13px;">Loading…</div>';

  void (async () => {
    try {
      const [mdResult, annResult] = await Promise.allSettled([
        api.fetchKbFileContent(repo, path),
        api.fetchKbAnnotation(repo, path),
      ]);

      if (token !== loadToken) return;

      const ann = annResult.status === 'fulfilled' ? (annResult.value || {}) : {};
      state.viewer.annotation = ann;

      if (mdResult.status === 'rejected') {
        throw new Error(mdResult.reason?.message || 'fetch failed');
      }
      const result = mdResult.value;
      if (result.error) throw new Error(result.error);

      const text = result.content;
      state.viewer.rawText = text;
      ui.fileSize.textContent = formatFileSize(text);

      if (typeof marked !== 'undefined') {
        await renderKbMdBody(text, ui.body);
      } else {
        ui.body.innerHTML = `<pre style="white-space:pre-wrap;font-size:13px">${escHtml(text)}</pre>`;
      }

      ui.btnEdit.style.display = '';
      const readerRoot = container.querySelector('.kb-reader') ?? container;
      initKbComments(readerRoot);
      initKbHighlightUI(readerRoot);

      api.fetchKbStatus(repo).then((data) => {
        if (token !== loadToken) return;
        if (!data.error && (data.total > 0 || data.ahead > 0)) {
          showPendingBadge('chore: update via viewer');
        }
      }).catch(() => {});
    } catch (e) {
      if (token !== loadToken) return;
      ui.body.innerHTML = `<div style="color:#7d4e00;padding:20px">Could not load file: ${escHtml(e.message)}</div>`;
      ui.btnEdit.style.display = 'none';
    }
  })();

  return { unmount };
}

// ── openKbDoc ──────────────────────────────────────────────────────────────
export async function openKbDoc(kbHit) {
  const { repo, path, url } = kbHit;

  state.viewer.entry = null;
  state.viewer.isKb = true;
  state.viewer.kbRepo = repo;
  state.viewer.kbPath = path;
  state.viewer.layer = 'raw';
  state.viewer.rawText = '';
  state.viewer.annotation = {};
  state.viewer.lang = null;

  const pathParts = (path || '').split('/');
  const fileName = pathParts.pop();
  const repoName = (repo || '').split('/').pop();
  document.getElementById('kb-md-panel-title').textContent =
    pathParts.length > 0 ? `${repoName}/.../${fileName}` : `${repoName}/${fileName}`;
  document.getElementById('kb-md-github-link').href = url || '#';
  document.getElementById('kb-btn-copy-http').dataset.url = url || '';
  document.getElementById('kb-btn-copy-http').dataset.tip = url || '';
  const localPath = state.ui.knowledgeCorpusRoot ? `${state.ui.knowledgeCorpusRoot}/${repoName}/${path}` : `${repoName}/${path}`;
  document.getElementById('kb-btn-copy-path').dataset.path = localPath;
  document.getElementById('kb-btn-copy-path').dataset.tip = localPath;
  document.getElementById('kb-md-file-size').textContent = '';

  const itermBtn = document.getElementById('kb-btn-open-iterm');
  itermBtn.style.display = '';
  itermBtn.onclick = async () => {
    itermBtn.disabled = true;
    try {
      const res = await api.openItermAt(repo);
      if (res.error) alert(`Failed to open terminal: ${res.error}`);
    } catch (e) {
      alert(`Failed to open terminal: ${e.message}`);
    } finally {
      itermBtn.disabled = false;
    }
  };

  const tagsBar = document.getElementById('md-tags-bar');
  if (tagsBar) tagsBar.style.display = 'none';

  const modal = document.getElementById('kb-md-modal');
  const body = document.getElementById('kb-md-body');
  body.innerHTML = '<div style="color:#8c959f;padding:20px;font-size:13px;">Loading…</div>';
  modal.style.display = 'flex';
  document.body.style.overflow = 'hidden';

  try {
    const [mdResult, annResult] = await Promise.allSettled([
      api.fetchKbFileContent(repo, path),
      api.fetchKbAnnotation(repo, path)
    ]);

    const ann = annResult.status === 'fulfilled' ? (annResult.value || {}) : {};
    state.viewer.annotation = ann;

    if (mdResult.status === 'rejected') {
      throw new Error(mdResult.reason?.message || 'fetch failed');
    }
    const result = mdResult.value;
    if (result.error) throw new Error(result.error);

    const text = result.content;
    state.viewer.rawText = text;

    const bytes = new Blob([text]).size;
    document.getElementById('kb-md-file-size').textContent = bytes < 1024
      ? `${bytes} B`
      : bytes < 1024 * 1024
        ? `${(bytes / 1024).toFixed(1)} KB`
        : `${(bytes / (1024 * 1024)).toFixed(1)} MB`;

    if (typeof marked !== 'undefined') {
      await renderKbMdBody(text);
    } else {
      body.innerHTML = `<pre style="white-space:pre-wrap;font-size:13px">${escHtml(text)}</pre>`;
    }

    document.getElementById('kb-btn-edit').style.display = '';

    initKbCommentEvents();
    initKbHighlightUI();

    // Restore badge if there are already pending changes
    api.fetchKbStatus(repo).then(data => {
      if (!data.error && (data.total > 0 || data.ahead > 0)) {
        _kbShowPendingBadge('chore: update via viewer');
      }
    }).catch(() => {});

    // Listen for dirty events from comments/highlights
    document.addEventListener('kb:dirty', _onKbDirty);
  } catch (e) {
    body.innerHTML = `<div style="color:#7d4e00;padding:20px">Could not load file: ${escHtml(e.message)}</div>`;
    document.getElementById('kb-btn-edit').style.display = 'none';
  }
}

// ── saveKbDoc ──────────────────────────────────────────────────────────────
export async function saveKbDoc() {
  const editArea = document.getElementById('kb-md-edit-area');
  const newContent = editArea.value;
  const btnSave = document.getElementById('kb-btn-save');
  btnSave.disabled = true;
  btnSave.textContent = 'Saving…';
  try {
    const originalContent = state.viewer.rawText;
    const data = await api.saveKbFile(state.viewer.kbRepo, state.viewer.kbPath, newContent);
    if (data.error) throw new Error(data.error);
    state.viewer.rawText = newContent;
    _kbExitEditMode();

    const body = document.getElementById('kb-md-body');
    if (typeof marked !== 'undefined') {
      await renderKbMdBody(newContent);
    } else {
      body.innerHTML = `<pre style="white-space:pre-wrap;font-size:13px">${escHtml(newContent)}</pre>`;
    }
    document.getElementById('kb-btn-edit').style.display = '';
    if (newContent !== originalContent) {
      _kbShowPendingBadge('update: edit via viewer');
      showKbReindexBtn(state.viewer.kbRepo);
    }
  } catch (e) {
    alert(`Save failed: ${e.message}`);
  } finally {
    btnSave.disabled = false;
    btnSave.textContent = '💾 Save';
  }
}

// ── openKbCommitDialog ────────────────────────────────────────────────────
async function openKbCommitDialog() {
  const dialog = document.getElementById('kb-commit-dialog');
  const result = document.getElementById('kb-commit-result');
  const okBtn = document.getElementById('kb-btn-commit-ok');

  result.textContent = '';
  result.style.color = '';
  document.getElementById('kb-commit-msg').value = _kbPendingMsg || '';
  okBtn.disabled = false;
  okBtn.textContent = 'Commit';
  _resetRevertAllBtn();
  dialog.classList.add('open');

  await _refreshKbCommitFileList();
}

// ── _refreshKbCommitFileList ───────────────────────────────────────────────
async function _refreshKbCommitFileList() {
  const fileList = document.getElementById('kb-commit-file-list');
  const okBtn = document.getElementById('kb-btn-commit-ok');
  const revertAllBtn = document.getElementById('kb-btn-revert-all');

  fileList.innerHTML = '<div style="font-size:12px;color:#8c959f;">Loading…</div>';

  try {
    const data = await api.fetchKbStatus(state.viewer.kbRepo);
    if (data.error) throw new Error(data.error);

    if (!data.total && !data.ahead) {
      fileList.innerHTML = '<div style="font-size:13px;color:#8c959f;padding:4px 0;">No changes to commit</div>';
      okBtn.disabled = true;
      if (revertAllBtn) revertAllBtn.disabled = true;
    } else {
      // Types that support per-file revert
      const REVERTABLE = new Set(['new', 'modified', 'deleted']);
      const GROUPS = [
        { key: 'new',        label: 'New' },
        { key: 'modified',   label: 'Modified' },
        { key: 'renamed',    label: 'Renamed' },
        { key: 'deleted',    label: 'Deleted' },
        { key: 'conflicted', label: 'Conflict' },
      ];

      const wrap = document.createElement('div');
      wrap.style.cssText = 'display:flex;flex-direction:column;gap:10px;';

      if (data.ahead) {
        const g = document.createElement('div');
        g.className = 'commit-file-group';
        g.innerHTML = `
          <div class="commit-file-group-title" style="color:#0969da;">Ready to push (${data.ahead} local commits)</div>
          <div class="commit-file-item" style="background:#ddf4ff;color:#0550ae;display:block;">${data.ahead} local commit(s) not yet pushed</div>`;
        wrap.appendChild(g);
      }

      for (const { key, label } of GROUPS) {
        if (!data[key]?.length) continue;
        const g = document.createElement('div');
        g.className = 'commit-file-group';
        const title = document.createElement('div');
        title.className = 'commit-file-group-title';
        title.textContent = `${label}（${data[key].length}）`;
        g.appendChild(title);

        for (const f of data[key]) {
          const row = document.createElement('div');
          row.className = `commit-file-item ${key}`;

          const pathEl = document.createElement('span');
          pathEl.className = 'commit-file-item-path';
          pathEl.textContent = f;
          row.appendChild(pathEl);

          if (REVERTABLE.has(key)) {
            const btn = document.createElement('button');
            btn.className = 'kb-revert-btn';
            btn.textContent = 'Revert';
            btn.title = `Revert changes to ${f}`;
            btn.addEventListener('click', () => _kbRevertFile(f, key, btn));
            row.appendChild(btn);
          }

          g.appendChild(row);
        }
        wrap.appendChild(g);
      }

      okBtn.disabled = false;
      okBtn.textContent = data.total ? 'Commit' : 'Push';
      if (revertAllBtn) revertAllBtn.disabled = false;
      fileList.innerHTML = '';
      fileList.appendChild(wrap);
    }
  } catch (e) {
    fileList.innerHTML = `<div style="font-size:12px;color:#cf222e;">Failed to get status: ${e.message}</div>`;
  }
}

// ── _kbRevertFile ─────────────────────────────────────────────────────────
async function _kbRevertFile(path, type, btn) {
  btn.disabled = true;
  btn.textContent = 'Reverting…';
  try {
    const data = await api.revertKbFile(state.viewer.kbRepo, path, type);
    if (data.error) throw new Error(data.error);
    await _refreshKbCommitFileList();
    // If the reverted path touches .knowledge_annotations (file or directory),
    // re-sync the current doc's in-memory annotation state.
    // Use a separate try/catch: annotation refresh failure must not mark revert as failed.
    if (state.viewer.kbPath && path.startsWith('.knowledge_annotations')) {
      try {
        const ann = await api.fetchKbAnnotation(state.viewer.kbRepo, state.viewer.kbPath);
        state.viewer.annotation = ann;
        renderKbComments(ann);
        void applyKbHighlights();
      } catch (_) { /* best-effort UI sync; revert already succeeded */ }
    }
    // If nothing left, auto-close
    const fileList = document.getElementById('kb-commit-file-list');
    const hasItems = fileList.querySelector('.commit-file-item');
    if (!hasItems) {
      _kbHidePendingBadge();
      setTimeout(closeKbCommitDialog, 800);
    }
  } catch (e) {
    btn.disabled = false;
    btn.textContent = 'Revert';
    btn.title = `Revert failed: ${e.message}`;
    btn.style.color = '#cf222e';
    btn.style.opacity = '1';
  }
}

// ── _kbRevertAll ──────────────────────────────────────────────────────────
let _revertAllConfirmTimer = null;

function _resetRevertAllBtn() {
  const btn = document.getElementById('kb-btn-revert-all');
  if (!btn) return;
  clearTimeout(_revertAllConfirmTimer);
  btn.textContent = 'Revert all changes';
  btn.classList.remove('confirm');
  btn.disabled = false;
}

async function _kbRevertAll(btn) {
  if (!btn.classList.contains('confirm')) {
    // First click — arm with 3 s auto-reset
    btn.classList.add('confirm');
    btn.textContent = '⚠ Revert all?';
    clearTimeout(_revertAllConfirmTimer);
    _revertAllConfirmTimer = setTimeout(_resetRevertAllBtn, 3000);
    return;
  }
  // Second click — execute
  clearTimeout(_revertAllConfirmTimer);
  btn.disabled = true;
  btn.textContent = 'Reverting…';
  try {
    const data = await api.revertKbFile(state.viewer.kbRepo);
    if (data.error) throw new Error(data.error);
    _kbHidePendingBadge();
    _resetRevertAllBtn();
    await _refreshKbCommitFileList();
    // Sync in-memory annotation after full revert
    if (state.viewer.kbPath) {
      try {
        const ann = await api.fetchKbAnnotation(state.viewer.kbRepo, state.viewer.kbPath);
        state.viewer.annotation = ann;
        renderKbComments(ann);
        void applyKbHighlights();
      } catch (_) { /* best-effort UI sync; revert already succeeded */ }
    }
  } catch (e) {
    _resetRevertAllBtn();
    const result = document.getElementById('kb-commit-result');
    if (result) { result.style.color = '#cf222e'; result.textContent = `✗ Revert failed: ${e.message}`; }
  }
}

function closeKbCommitDialog() {
  document.getElementById('kb-commit-dialog').classList.remove('open');
  document.getElementById('kb-btn-commit-ok').textContent = 'Commit';
  _resetRevertAllBtn();
}

// ── doKbCommit ─────────────────────────────────────────────────────────────
async function doKbCommit() {
  const btn = document.getElementById('kb-btn-commit-ok');
  const result = document.getElementById('kb-commit-result');
  const msg = document.getElementById('kb-commit-msg').value.trim() || 'chore: update via viewer';
  btn.disabled = true;
  result.textContent = 'Committing…';
  result.style.color = '#8c959f';

  try {
    const data = await api.commitKbFile(state.viewer.kbRepo, msg);
    if (data.error) throw new Error(data.error + (data.stderr ? '\n' + data.stderr : ''));
    result.style.color = '#1a7f37';
    result.textContent = '✓ Committed and pushed';
    _kbHidePendingBadge();
    showKbReindexBtn(state.viewer.kbRepo);
    setTimeout(closeKbCommitDialog, 1500);
  } catch (e) {
    result.style.color = '#cf222e';
    result.textContent = `✗ ${e.message}`;
    btn.disabled = false;
  }
}

// ── showKbReindexBtn ───────────────────────────────────────────────────────
export function showKbReindexBtn(repo) {
  let btn = document.getElementById('kb-btn-reindex');
  if (!btn) {
    btn = document.createElement('button');
    btn.id = 'kb-btn-reindex';
    btn.className = 'md-header-btn';
    const closeBtn = document.getElementById('kb-md-close');
    closeBtn.parentNode.insertBefore(btn, closeBtn);
  }
  btn.textContent = '↺ Rebuild index';
  btn.title = 'Rebuild search index for this library';
  btn.disabled = false;
  btn.style.display = '';
  btn.onclick = async () => {
    btn.disabled = true;
    btn.textContent = 'Rebuilding…';
    try {
      const res = await api.reindexKbRepo(repo);
      if (res.error) throw new Error(res.error);
      const poll = setInterval(async () => {
        try {
          const status = await api.getReindexStatus();
          if (status.status === 'done') {
            clearInterval(poll);
            btn.textContent = '✓ Rebuilt';
            btn.disabled = false;
            setTimeout(() => { btn.style.display = 'none'; }, 2000);
          } else if (status.status === 'error') {
            clearInterval(poll);
            btn.textContent = 'Rebuild failed';
            btn.disabled = false;
            btn.title = status.log || 'Unknown error';
          }
        } catch (_) {}
      }, 2000);
    } catch (e) {
      btn.textContent = 'Rebuild failed';
      btn.disabled = false;
      btn.title = e.message;
    }
  };
}

// ── closeKbModal ───────────────────────────────────────────────────────────
export function closeKbModal() {
  document.getElementById('kb-md-modal').style.display = 'none';
  document.removeEventListener('kb:dirty', _onKbDirty);
  state.viewer.isKb = false;
  state.viewer.kbRepo = null;
  state.viewer.kbPath = null;
  state.viewer.annotation = {};
  document.body.style.overflow = '';
  _kbHidePendingBadge();
}

// ── Pending badge helpers ──────────────────────────────────────────────────
let _kbPendingMsg = '';

function _onKbDirty(e) {
  _kbShowPendingBadge(e.detail?.msg || 'chore: update via viewer');
}

function _kbShowPendingBadge(msg) {
  _kbPendingMsg = msg;
  const btn = document.getElementById('kb-btn-pending');
  if (btn) btn.style.display = '';
}

function _kbHidePendingBadge() {
  _kbPendingMsg = '';
  const btn = document.getElementById('kb-btn-pending');
  if (btn) btn.style.display = 'none';
}
function _kbEnterEditMode() {
  const body = document.getElementById('kb-md-body');
  const editArea = document.getElementById('kb-md-edit-area');
  setDocEditMode({
    bodyEl: body,
    editAreaEl: editArea,
    text: state.viewer.rawText,
    editing: true,
  });
  document.getElementById('kb-btn-save').style.display = '';
  document.getElementById('kb-btn-cancel-edit').style.display = '';
  document.getElementById('kb-btn-edit').style.display = 'none';
  resetEditAreaScroll(editArea, { focus: true });
}

function _kbExitEditMode() {
  const body = document.getElementById('kb-md-body');
  const editArea = document.getElementById('kb-md-edit-area');
  setDocEditMode({ bodyEl: body, editAreaEl: editArea, editing: false });
  document.getElementById('kb-btn-save').style.display = 'none';
  document.getElementById('kb-btn-cancel-edit').style.display = 'none';
  document.getElementById('kb-btn-edit').style.display = '';
}

// ── Edit mode helpers ──────────────────────────────────────────────────────
document.getElementById('kb-md-close')?.addEventListener('click', closeKbModal);
document.getElementById('kb-md-backdrop')?.addEventListener('click', closeKbModal);

document.addEventListener('keydown', e => {
  if (e.key !== 'Escape') return;
  const kbModal = document.getElementById('kb-md-modal');
  if (!kbModal || kbModal.style.display === 'none') return;
  const kbCommitDialog = document.getElementById('kb-commit-dialog');
  if (kbCommitDialog && kbCommitDialog.classList.contains('open')) return;
  const kbDialog = document.getElementById('kb-comment-dialog');
  if (kbDialog && kbDialog.style.display !== 'none') return;
  closeKbModal();
});

document.getElementById('kb-btn-edit')?.addEventListener('click', _kbEnterEditMode);

document.getElementById('kb-btn-save')?.addEventListener('click', saveKbDoc);

document.getElementById('kb-btn-cancel-edit')?.addEventListener('click', () => {
  _kbExitEditMode();
});

// ── Commit dialog event listeners ───────────────────────────────────────
document.getElementById('kb-btn-pending')?.addEventListener('click', openKbCommitDialog);
document.getElementById('kb-btn-commit-cancel')?.addEventListener('click', closeKbCommitDialog);
document.getElementById('kb-btn-commit-ok')?.addEventListener('click', doKbCommit);
document.getElementById('kb-btn-revert-all')?.addEventListener('click', e => _kbRevertAll(e.currentTarget));
document.getElementById('kb-commit-dialog')?.addEventListener('click', e => {
  if (e.target === document.getElementById('kb-commit-dialog')) closeKbCommitDialog();
});
document.getElementById('kb-commit-msg')?.addEventListener('keydown', e => {
  if (e.key === 'Enter') { e.preventDefault(); doKbCommit(); }
  if (e.key === 'Escape') { e.stopPropagation(); closeKbCommitDialog(); }
});

document.getElementById('kb-btn-copy-http')?.addEventListener('click', e => {
  const btn = e.currentTarget;
  const url = btn.dataset.url || '';
  if (!url) return;
  navigator.clipboard.writeText(url).then(() => {
    const orig = btn.textContent;
    btn.textContent = '✓';
    setTimeout(() => { btn.textContent = orig; }, 1200);
  }).catch(() => {});
});

document.getElementById('kb-btn-copy-path')?.addEventListener('click', e => {
  const btn = e.currentTarget;
  const path = btn.dataset.path || '';
  if (!path) return;
  navigator.clipboard.writeText(path).then(() => {
    const orig = btn.textContent;
    btn.textContent = '✓';
    setTimeout(() => { btn.textContent = orig; }, 1200);
  }).catch(() => {});
});
