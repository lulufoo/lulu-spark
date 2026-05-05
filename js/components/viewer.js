import { state, getEntryId, loadDiffStatus } from '../state.js'
import { REPO } from '../constants.js'
import { escHtml, filenameFromPath, slugToTitle } from '../utils.js'
import * as api from '../api.js'
import { updateTitlesInDOM, updateDiffInDOM } from './cards.js'
import { renderLinksBar } from './links-bar.js'
import { renderComments } from './comments.js'
import { openDeleteDialog } from './modals/delete-dialog.js'

// ── resolveRelativeLink ────────────────────────────────────────────────────

function resolveRelativeLink(href, layer, commonPath) {
  try {
    const base = `http://x/${layer}/${commonPath}`;
    const resolved = new URL(href, base);
    const repoPath = resolved.pathname.slice(1);
    return `${REPO}/${repoPath}`;
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

// ── renderDocBody ──────────────────────────────────────────────────────────

export function renderDocBody(text, layer, commonPath) {
  const body = document.getElementById('md-body');
  if (typeof marked !== 'undefined') {
    body.innerHTML = marked.parse(text);
  } else {
    body.innerHTML = `<pre style="white-space:pre-wrap;font-size:13px">${escHtml(text)}</pre>`;
  }
  postProcessLinks(body, layer, commonPath);
  document.getElementById('btn-edit').style.display = '';
  renderLinksBar(state.viewer.entry);
  renderComments(state.viewer.annotation, layer, state.viewer.entry);
  const zone = document.createElement('div');
  zone.className = 'md-body-delete-zone';
  const delBtn = document.createElement('button');
  delBtn.id = 'btn-delete';
  delBtn.textContent = '🗑 删除此条目';
  delBtn.title = '删除此条目的所有关联文件（raw / distilled / trace / digest / diagnose）';
  delBtn.addEventListener('click', () => openDeleteDialog());
  zone.appendChild(delBtn);
  body.appendChild(zone);
}

// ── openDoc ────────────────────────────────────────────────────────────────

export async function openDoc(entry, layer = 'raw') {
  state.viewer.entry = entry;
  state.viewer.layer = layer;
  state.viewer.annotation = {};
  exitEditMode(false);
  hideCommitBar();

  const modal = document.getElementById('md-modal');
  const body = document.getElementById('md-body');
  document.getElementById('md-panel-title').textContent = filenameFromPath(entry.common_path).replace(/\.md$/, '');
  const githubUrl = `${REPO}/${layer}/${entry.common_path}`;
  document.getElementById('md-github-link').href = githubUrl;
  document.getElementById('btn-copy-http').dataset.url = githubUrl;
  document.getElementById('btn-copy-http').dataset.tip = githubUrl;
  const relPath = `${layer}/${entry.common_path}`;
  const fullPath = state.ui.archiveRoot ? `${state.ui.archiveRoot}/${relPath}` : relPath;
  document.getElementById('btn-copy-path').dataset.tip = fullPath;
  document.getElementById('md-file-size').textContent = '';
  body.innerHTML = '<div style="color:#8c959f;padding:20px;font-size:13px;">加载中…</div>';
  modal.style.display = 'flex';
  document.body.style.overflow = 'hidden';

  const [mdResult, annResult] = await Promise.allSettled([
    api.fetchFileContent(layer, entry.common_path),
    api.fetchAnnotation(entry.common_path).catch(() => ({}))
  ]);

  if (annResult.status === 'fulfilled') state.viewer.annotation = annResult.value || {};

  if (mdResult.status === 'rejected') {
    body.innerHTML = `<div style="color:#7d4e00;padding:20px">无法加载文件：${escHtml(mdResult.reason.message)}</div>`;
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
  renderDocBody(text, layer, entry.common_path);
  const hasDiff = state.index.diffStatus.get(`${layer}/${entry.common_path}`);
  document.getElementById('btn-panel-commit').style.display = hasDiff ? '' : 'none';
}

// viewer.js exposes openDoc on window so cards.js (window.openDoc) can reach it
window.openDoc = openDoc;

// ── Edit mode ──────────────────────────────────────────────────────────────

export function enterEditMode() {
  const editArea = document.getElementById('md-edit-area');
  const body = document.getElementById('md-body');
  editArea.value = state.viewer.rawText;
  body.style.display = 'none';
  editArea.style.display = '';
  requestAnimationFrame(() => { editArea.scrollTop = 0; });
  editArea.focus();
  document.getElementById('btn-edit').style.display = 'none';
  document.getElementById('btn-add-comment').style.display = 'none';
  document.getElementById('btn-panel-commit').style.display = 'none';
  document.getElementById('btn-save').style.display = '';
  document.getElementById('btn-cancel-edit').style.display = '';
  document.getElementById('md-github-link').style.display = 'none';
  hideCommitBar();
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
  document.getElementById('btn-panel-commit').style.display = 'none';
  document.getElementById('md-github-link').style.display = '';
  if (rerender && state.viewer.entry) {
    renderDocBody(state.viewer.rawText, state.viewer.layer, state.viewer.entry.common_path);
  }
}

export async function saveDoc() {
  if (!state.viewer.entry) return;
  const editArea = document.getElementById('md-edit-area');
  const newContent = editArea.value;
  const btnSave = document.getElementById('btn-save');
  btnSave.disabled = true;
  btnSave.textContent = '保存中…';

  try {
    const data = await api.saveFile(state.viewer.layer, state.viewer.entry.common_path, newContent);
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
    showCommitBar();
  } catch (e) {
    alert(`保存失败：${e.message}\n\n请确认已通过 python3 server.py 启动服务器。`);
  } finally {
    btnSave.disabled = false;
    btnSave.textContent = '💾 保存';
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
  if (!state.viewer.entry) return;
  const msg = document.getElementById('md-commit-msg').value.trim() || 'update: edit via viewer';
  const resultEl = document.getElementById('md-commit-result');
  const btn = document.getElementById('btn-commit-file');
  btn.disabled = true;
  btn.textContent = '提交中…';
  resultEl.style.color = '#57606a';
  resultEl.textContent = '';

  try {
    const filePath = `${state.viewer.layer}/${state.viewer.entry.common_path}`;
    const data = await api.commitFiles(msg, [filePath]);
    if (data.error) throw new Error(data.error + (data.stderr ? '\n' + data.stderr : ''));

    resultEl.style.color = '#1a7f37';
    resultEl.textContent = '✓ 已推送！';
    await loadDiffStatus();
    updateDiffInDOM();
    document.dispatchEvent(new CustomEvent('cta:reload'));
    setTimeout(hideCommitBar, 1500);
  } catch (e) {
    resultEl.style.color = '#cf222e';
    resultEl.textContent = `✗ ${e.message}`;
  } finally {
    btn.disabled = false;
    btn.textContent = '↑ 提交';
  }
}

// ── Close viewer ───────────────────────────────────────────────────────────

export function closeModal() {
  document.getElementById('md-modal').style.display = 'none';
  document.body.style.overflow = '';
  exitEditMode(false);
  hideCommitBar();
}

document.getElementById('md-close').addEventListener('click', closeModal);
document.getElementById('md-backdrop').addEventListener('click', closeModal);
document.addEventListener('keydown', e => { if (e.key === 'Escape') closeModal(); });

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
  const relPath = `${state.viewer.layer}/${state.viewer.entry.common_path}`;
  const fullPath = state.ui.archiveRoot ? `${state.ui.archiveRoot}/${relPath}` : relPath;
  navigator.clipboard.writeText(fullPath).then(() => {
    const btn = document.getElementById('btn-copy-path');
    btn.textContent = '✓';
    setTimeout(() => { btn.textContent = '📂'; }, 1200);
  });
});

