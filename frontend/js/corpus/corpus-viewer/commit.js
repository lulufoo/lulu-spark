import { state } from '../../host/state.js'
import * as api from '../../host/api.js'
import { renderKbComments } from '../corpus-comments.js'
import { applyKbHighlights } from './highlight.js'
import { kbHidePendingBadge, kbPendingMsg, kbShowPendingBadge, showKbReindexBtn } from './chrome.js'

// ── openKbCommitDialog ────────────────────────────────────────────────────
export async function openKbCommitDialog() {
  const dialog = document.getElementById('kb-commit-dialog');
  const result = document.getElementById('kb-commit-result');
  const okBtn = document.getElementById('kb-btn-commit-ok');

  result.textContent = '';
  result.style.color = '';
  document.getElementById('kb-commit-msg').value = kbPendingMsg() || '';
  okBtn.disabled = false;
  okBtn.textContent = 'Commit';
  _resetRevertAllBtn();
  dialog.classList.add('open');

  await _refreshKbCommitFileList();
}

// ── _refreshKbCommitFileList ───────────────────────────────────────────────
export async function _refreshKbCommitFileList() {
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
export async function _kbRevertFile(path, type, btn) {
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
      kbHidePendingBadge();
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

export function _resetRevertAllBtn() {
  const btn = document.getElementById('kb-btn-revert-all');
  if (!btn) return;
  clearTimeout(_revertAllConfirmTimer);
  btn.textContent = 'Revert all changes';
  btn.classList.remove('confirm');
  btn.disabled = false;
}

export async function _kbRevertAll(btn) {
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
    kbHidePendingBadge();
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

export function closeKbCommitDialog() {
  document.getElementById('kb-commit-dialog').classList.remove('open');
  document.getElementById('kb-btn-commit-ok').textContent = 'Commit';
  _resetRevertAllBtn();
}

// ── doKbCommit ─────────────────────────────────────────────────────────────
export async function doKbCommit() {
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
    kbHidePendingBadge();
    showKbReindexBtn(state.viewer.kbRepo);
    setTimeout(closeKbCommitDialog, 1500);
  } catch (e) {
    result.style.color = '#cf222e';
    result.textContent = `✗ ${e.message}`;
    btn.disabled = false;
  }
}
