import { state } from '../state.js'
import { escHtml } from '../utils.js'
import * as api from '../api.js'
import { renderKbComments, initKbCommentEvents } from './kb-comments.js'
import { applyKbHighlights, initKbHighlightUI } from './kb-highlights.js'

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
  const localPath = state.ui.kbRoot ? `${state.ui.kbRoot}/${repoName}/${path}` : `${repoName}/${path}`;
  document.getElementById('kb-btn-copy-path').dataset.path = localPath;
  document.getElementById('kb-btn-copy-path').dataset.tip = localPath;
  document.getElementById('kb-md-file-size').textContent = '';

  const itermBtn = document.getElementById('kb-btn-open-iterm');
  itermBtn.style.display = '';
  itermBtn.onclick = async () => {
    itermBtn.disabled = true;
    try {
      const res = await api.openItermAt(repo);
      if (res.error) alert(`打开终端失败：${res.error}`);
    } catch (e) {
      alert(`打开终端失败：${e.message}`);
    } finally {
      itermBtn.disabled = false;
    }
  };

  const modal = document.getElementById('kb-md-modal');
  const body = document.getElementById('kb-md-body');
  body.innerHTML = '<div style="color:#8c959f;padding:20px;font-size:13px;">加载中…</div>';
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
      body.innerHTML = marked.parse(text);
    } else {
      body.innerHTML = `<pre style="white-space:pre-wrap;font-size:13px">${escHtml(text)}</pre>`;
    }
    postProcessKbLinks(body);

    document.getElementById('kb-btn-edit').style.display = '';

    renderKbComments(ann);
    applyKbHighlights(ann);
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
    body.innerHTML = `<div style="color:#7d4e00;padding:20px">无法加载文件：${escHtml(e.message)}</div>`;
    document.getElementById('kb-btn-edit').style.display = 'none';
  }
}

// ── saveKbDoc ──────────────────────────────────────────────────────────────
export async function saveKbDoc() {
  const editArea = document.getElementById('kb-md-edit-area');
  const newContent = editArea.value;
  const btnSave = document.getElementById('kb-btn-save');
  btnSave.disabled = true;
  btnSave.textContent = '保存中…';
  try {
    const originalContent = state.viewer.rawText;
    const data = await api.saveKbFile(state.viewer.kbRepo, state.viewer.kbPath, newContent);
    if (data.error) throw new Error(data.error);
    state.viewer.rawText = newContent;
    _kbExitEditMode();

    const body = document.getElementById('kb-md-body');
    if (typeof marked !== 'undefined') {
      body.innerHTML = marked.parse(newContent);
    } else {
      body.innerHTML = `<pre style="white-space:pre-wrap;font-size:13px">${escHtml(newContent)}</pre>`;
    }
    postProcessKbLinks(body);
    renderKbComments(state.viewer.annotation);
    applyKbHighlights(state.viewer.annotation);
    document.getElementById('kb-btn-edit').style.display = '';
    if (newContent !== originalContent) {
      _kbShowPendingBadge('update: edit via viewer');
      showKbReindexBtn(state.viewer.kbRepo);
    }
  } catch (e) {
    alert(`保存失败：${e.message}\n\n请确认已通过 python3 server.py 启动服务器。`);
  } finally {
    btnSave.disabled = false;
    btnSave.textContent = '💾 保存';
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
  okBtn.textContent = '提交';
  _resetRevertAllBtn();
  dialog.classList.add('open');

  await _refreshKbCommitFileList();
}

// ── _refreshKbCommitFileList ───────────────────────────────────────────────
async function _refreshKbCommitFileList() {
  const fileList = document.getElementById('kb-commit-file-list');
  const okBtn = document.getElementById('kb-btn-commit-ok');
  const revertAllBtn = document.getElementById('kb-btn-revert-all');

  fileList.innerHTML = '<div style="font-size:12px;color:#8c959f;">加载中…</div>';

  try {
    const data = await api.fetchKbStatus(state.viewer.kbRepo);
    if (data.error) throw new Error(data.error);

    if (!data.total && !data.ahead) {
      fileList.innerHTML = '<div style="font-size:13px;color:#8c959f;padding:4px 0;">没有待提交的变更</div>';
      okBtn.disabled = true;
      if (revertAllBtn) revertAllBtn.disabled = true;
    } else {
      // Types that support per-file revert
      const REVERTABLE = new Set(['new', 'modified', 'deleted']);
      const GROUPS = [
        { key: 'new',        label: '新增' },
        { key: 'modified',   label: '修改' },
        { key: 'renamed',    label: '重命名' },
        { key: 'deleted',    label: '删除' },
        { key: 'conflicted', label: '冲突' },
      ];

      const wrap = document.createElement('div');
      wrap.style.cssText = 'display:flex;flex-direction:column;gap:10px;';

      if (data.ahead) {
        const g = document.createElement('div');
        g.className = 'commit-file-group';
        g.innerHTML = `
          <div class="commit-file-group-title" style="color:#0969da;">待推送（${data.ahead} 个本地提交）</div>
          <div class="commit-file-item" style="background:#ddf4ff;color:#0550ae;display:block;">本地已有 ${data.ahead} 个提交尚未推送到远端</div>`;
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
            btn.textContent = '撤销';
            btn.title = `撤销对 ${f} 的修改`;
            btn.addEventListener('click', () => _kbRevertFile(f, key, btn));
            row.appendChild(btn);
          }

          g.appendChild(row);
        }
        wrap.appendChild(g);
      }

      okBtn.disabled = false;
      okBtn.textContent = data.total ? '提交' : '推送';
      if (revertAllBtn) revertAllBtn.disabled = false;
      fileList.innerHTML = '';
      fileList.appendChild(wrap);
    }
  } catch (e) {
    fileList.innerHTML = `<div style="font-size:12px;color:#cf222e;">获取状态失败：${e.message}</div>`;
  }
}

// ── _kbRevertFile ─────────────────────────────────────────────────────────
async function _kbRevertFile(path, type, btn) {
  btn.disabled = true;
  btn.textContent = '撤销中…';
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
        applyKbHighlights(ann);
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
    btn.textContent = '撤销';
    btn.title = `撤销失败：${e.message}`;
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
  btn.textContent = '撤销全部修改';
  btn.classList.remove('confirm');
  btn.disabled = false;
}

async function _kbRevertAll(btn) {
  if (!btn.classList.contains('confirm')) {
    // First click — arm with 3 s auto-reset
    btn.classList.add('confirm');
    btn.textContent = '⚠ 确认撤销所有？';
    clearTimeout(_revertAllConfirmTimer);
    _revertAllConfirmTimer = setTimeout(_resetRevertAllBtn, 3000);
    return;
  }
  // Second click — execute
  clearTimeout(_revertAllConfirmTimer);
  btn.disabled = true;
  btn.textContent = '撤销中…';
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
        applyKbHighlights(ann);
      } catch (_) { /* best-effort UI sync; revert already succeeded */ }
    }
  } catch (e) {
    _resetRevertAllBtn();
    const result = document.getElementById('kb-commit-result');
    if (result) { result.style.color = '#cf222e'; result.textContent = `✗ 撤销失败：${e.message}`; }
  }
}

function closeKbCommitDialog() {
  document.getElementById('kb-commit-dialog').classList.remove('open');
  document.getElementById('kb-btn-commit-ok').textContent = '提交';
  _resetRevertAllBtn();
}

// ── doKbCommit ─────────────────────────────────────────────────────────────
async function doKbCommit() {
  const btn = document.getElementById('kb-btn-commit-ok');
  const result = document.getElementById('kb-commit-result');
  const msg = document.getElementById('kb-commit-msg').value.trim() || 'chore: update via viewer';
  btn.disabled = true;
  result.textContent = '提交中…';
  result.style.color = '#8c959f';

  try {
    const data = await api.commitKbFile(state.viewer.kbRepo, msg);
    if (data.error) throw new Error(data.error + (data.stderr ? '\n' + data.stderr : ''));
    result.style.color = '#1a7f37';
    result.textContent = '✓ 提交并推送成功';
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
  btn.textContent = '↺ 重建索引';
  btn.title = '重建此知识库的搜索索引';
  btn.disabled = false;
  btn.style.display = '';
  btn.onclick = async () => {
    btn.disabled = true;
    btn.textContent = '重建中…';
    try {
      const res = await api.reindexKbRepo(repo);
      if (res.error) throw new Error(res.error);
      const poll = setInterval(async () => {
        try {
          const status = await api.getReindexStatus();
          if (status.status === 'done') {
            clearInterval(poll);
            btn.textContent = '✓ 已重建';
            btn.disabled = false;
            setTimeout(() => { btn.style.display = 'none'; }, 2000);
          } else if (status.status === 'error') {
            clearInterval(poll);
            btn.textContent = '重建失败';
            btn.disabled = false;
            btn.title = status.log || '未知错误';
          }
        } catch (_) {}
      }, 2000);
    } catch (e) {
      btn.textContent = '重建失败';
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
  editArea.value = state.viewer.rawText;
  body.style.display = 'none';
  editArea.style.display = '';
  document.getElementById('kb-btn-save').style.display = '';
  document.getElementById('kb-btn-cancel-edit').style.display = '';
  document.getElementById('kb-btn-edit').style.display = 'none';
  editArea.focus();
}

function _kbExitEditMode() {
  const body = document.getElementById('kb-md-body');
  const editArea = document.getElementById('kb-md-edit-area');
  body.style.display = '';
  editArea.style.display = 'none';
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
