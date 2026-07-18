import { state } from '../state.js'
import * as api from '../api.js'
import { reorderKbComments } from '../api.js'
import {
  ensureKbComments,
  swapAdjacent,
  validateCommentIdsForReorder,
} from '../comment-reorder.js'
import { nowTs } from '../utils.js'
import { confirmDeleteComment, removeKbComment } from './comment-delete.js'
import {
  pasteIntoCommentEditor,
  prepareCommentMarkdown,
  renderCommentMarkdown,
} from '../comment-markdown.js'
import { renderMermaidBlocks } from '../mermaid-render.js'

/** @type {HTMLElement | null} */
let _kbCommentsRoot = null;
/** @type {Array<() => void>} */
const _kbCommentsCleanups = [];
let _kbDialogEventsBound = false;

function kbCommentsBodyEl() {
  if (_kbCommentsRoot) {
    return _kbCommentsRoot.querySelector('.kb-reader-body');
  }
  return document.getElementById('kb-md-body');
}

function kbFloatNavEl() {
  if (_kbCommentsRoot) {
    return _kbCommentsRoot.querySelector('.kb-comment-float-nav');
  }
  return document.getElementById('kb-comment-float-nav');
}

// ── Preview tip (shared DOM element) ──────────────────────────────────────
const _tip = () => document.getElementById('comment-preview-tip');

function _showKbTip(text, e) {
  const tip = _tip();
  if (!tip) return;
  const preview = text.length > 600 ? text.slice(0, 600) + '\n\n…' : text;
  const inner = renderCommentMarkdown(preview);
  tip.innerHTML = `<div class="comment-item-text">${inner}</div>`;
  tip.style.display = 'block';
  _moveKbTip(e);
}

function _moveKbTip(e) {
  const tip = _tip();
  if (!tip || tip.style.display === 'none') return;
  const GAP = 12;
  let x = e.clientX + GAP;
  let y = e.clientY + GAP;
  const tw = tip.offsetWidth;
  const th = tip.offsetHeight;
  if (x + tw > window.innerWidth - 8) x = e.clientX - tw - GAP;
  if (y + th > window.innerHeight - 8) y = e.clientY - th - GAP;
  tip.style.left = x + 'px';
  tip.style.top  = y + 'px';
}

function _hideKbTip() {
  const tip = _tip();
  if (tip) tip.style.display = 'none';
}

// ── Float nav ──────────────────────────────────────────────────────────────
let _kbFloatNavObserver = null;

function updateKbFloatNav(comments) {
  const nav = kbFloatNavEl();
  if (!nav) return;
  nav.innerHTML = '';
  nav.style.display = 'none';
  if (_kbFloatNavObserver) {
    _kbFloatNavObserver.disconnect();
    _kbFloatNavObserver = null;
  }
  if (!comments || !comments.length) return;

  comments.forEach((c, i) => {
    const btn = document.createElement('button');
    btn.className = 'comment-float-btn';
    btn.textContent = String(i + 1);
    btn.title = `Edit comment ${i + 1}`;
    btn.addEventListener('click', () => openKbCommentDialog(c, i));
    btn.addEventListener('mouseenter', e => _showKbTip(c.text, e));
    btn.addEventListener('mousemove',  e => _moveKbTip(e));
    btn.addEventListener('mouseleave', () => _hideKbTip());
    nav.appendChild(btn);
  });

  const bar = kbCommentsBodyEl()?.querySelector('.md-comments-bar');
  if (!bar) return;
  const mdBody = kbCommentsBodyEl();
  _kbFloatNavObserver = new IntersectionObserver(
    ([e]) => { nav.style.display = e.isIntersecting ? 'none' : 'flex'; },
    { root: mdBody, threshold: 0 }
  );
  _kbFloatNavObserver.observe(bar);
}
// ── renderKbComments ───────────────────────────────────────────────────────
export function renderKbComments(annotation) {
  const body = kbCommentsBodyEl();
  body?.querySelector('.md-comments-bar')?.remove();

  const comments = annotation?.comments || [];
  if (!comments.length) {
    // BUG3 fix: clear float nav even when no comments remain
    updateKbFloatNav([]);
    return;
  }

  const bodyEl = kbCommentsBodyEl();
  if (!bodyEl) return;

  const bar = document.createElement('div');
  bar.id = 'kb-md-comments-bar';
  bar.className = 'md-comments-bar';

  comments.forEach((c, i) => {
    bar.appendChild(buildKbCommentItem(c, i, comments));
  });

  bodyEl.prepend(bar);
  // Must call after bar is in DOM so IntersectionObserver can find it
  updateKbFloatNav(comments);
}

async function moveKbComment(index, delta) {
  const { kbRepo, kbPath, annotation } = state.viewer;
  const arr = ensureKbComments(annotation);
  if (!swapAdjacent(arr, index, delta)) return;
  renderKbComments(annotation);
  const check = validateCommentIdsForReorder(arr);
  if (!check.ok) {
    alert(check.error);
    return;
  }
  try {
    const data = await reorderKbComments(kbRepo, kbPath, check.ids);
    if (data?.ok !== true) throw new Error(data?.error || 'failed');
    document.dispatchEvent(new CustomEvent('kb:dirty', { detail: { msg: 'chore: reorder annotations' } }));
  } catch (e) {
    alert(`Reorder save failed: ${e.message}`);
  }
}

function buildKbCommentItem(comment, index, comments) {
  const item = document.createElement('div');
  item.className = 'comment-item';
  item.dataset.cid = comment.id;

  const prefixEl = document.createElement('span');
  prefixEl.className = 'comment-item-prefix';
  prefixEl.textContent = `NOTE ${index + 1}:`;

  const textEl = document.createElement('div');
  textEl.className = 'comment-item-text';
  if (typeof marked !== 'undefined') {
    textEl.innerHTML = renderCommentMarkdown(comment.text || '');
  } else {
    textEl.textContent = comment.text || '';
  }

  const tsEl = document.createElement('span');
  tsEl.className = 'comment-item-ts';
  const ts = comment.ts || '';
  if (ts.length >= 12) {
    tsEl.textContent = `${ts.slice(0,4)}-${ts.slice(4,6)}-${ts.slice(6,8)} ${ts.slice(8,10)}:${ts.slice(10,12)}`;
  }

  const delX = document.createElement('button');
  delX.type = 'button';
  delX.className = 'comment-item-del-x';
  delX.title = 'Delete';
  delX.textContent = '×';
  delX.addEventListener('click', async () => {
    if (!(await confirmDeleteComment())) return;
    try {
      await removeKbComment(comment);
      renderKbComments(state.viewer.annotation);
      document.dispatchEvent(new CustomEvent('kb:dirty', { detail: { msg: 'chore: update annotations' } }));
    } catch (e) {
      alert(`Delete failed: ${e.message}`);
    }
  });

  const actionsEl = document.createElement('div');
  actionsEl.className = 'comment-item-actions';

  const editBtn = document.createElement('button');
  editBtn.type = 'button';
  editBtn.className = 'comment-item-action-btn';
  editBtn.textContent = 'Edit';
  editBtn.addEventListener('click', () => openKbCommentDialog(comment, index));

  actionsEl.append(editBtn);

  if (comments && index > 0) {
    const upBtn = document.createElement('button');
    upBtn.type = 'button';
    upBtn.className = 'comment-item-action-btn comment-item-order-btn';
    upBtn.title = 'Move up';
    upBtn.textContent = '↑';
    upBtn.addEventListener('click', () => moveKbComment(index, -1));
    actionsEl.append(upBtn);
  }

  if (comments && index < comments.length - 1) {
    const downBtn = document.createElement('button');
    downBtn.type = 'button';
    downBtn.className = 'comment-item-action-btn comment-item-order-btn';
    downBtn.title = 'Move down';
    downBtn.textContent = '↓';
    downBtn.addEventListener('click', () => moveKbComment(index, 1));
    actionsEl.append(downBtn);
  }

  item.append(delX, prefixEl, textEl, tsEl, actionsEl);
  return item;
}

// ── Dialog state ───────────────────────────────────────────────────────────

let _editingComment = null;
let _editingIndex = null;

// ── Tab helpers ────────────────────────────────────────────────────────────
function _resetKbDialogTabs() {
  document.querySelectorAll('#kb-comment-dialog .comment-tab-btn').forEach(b => {
    b.classList.toggle('active', b.dataset.tab === 'edit');
  });
  const editorBox = document.getElementById('kb-comment-editor-box');
  const previewPane = document.getElementById('kb-comment-preview-pane');
  if (editorBox) editorBox.style.display = '';
  if (previewPane) previewPane.style.display = 'none';
}

// ── openKbCommentDialog ────────────────────────────────────────────────────
export function openKbCommentDialog(editComment = null, noteIndex = null) {
  _editingComment = editComment;
  _editingIndex = noteIndex;

  const dialog = document.getElementById('kb-comment-dialog');
  const title = document.getElementById('kb-comment-dialog-title');
  const content = document.getElementById('kb-comment-dialog-content');
  if (!dialog || !content) return;

  title.textContent = editComment ? '💬 Edit comment' : '💬 Add comment';
  content.textContent = editComment?.text || '';
  dialog.style.display = 'flex';
  _resetKbDialogTabs();
  content.focus();

  // Move cursor to end
  const range = document.createRange();
  const sel = window.getSelection();
  range.selectNodeContents(content);
  range.collapse(false);
  sel.removeAllRanges();
  sel.addRange(range);
}

// ── closeKbCommentDialog ───────────────────────────────────────────────────
export function closeKbCommentDialog() {
  const dialog = document.getElementById('kb-comment-dialog');
  if (dialog) dialog.style.display = 'none';
  _editingComment = null;
  _editingIndex = null;
}

// ── saveKbComment ──────────────────────────────────────────────────────────
export async function saveKbComment() {
  const content = document.getElementById('kb-comment-dialog-content');
  if (!content) return;
  const text = prepareCommentMarkdown(content.textContent.trim());
  if (!text) return;

  const { kbRepo, kbPath, annotation } = state.viewer;
  if (!kbRepo || !kbPath) return;

  try {
    if (_editingComment) {
      // edit
      const data = await api.updateKbComment(kbRepo, kbPath, { id: _editingComment.id, text }, nowTs());
      if (!data.ok) throw new Error(data.error || 'failed');
      if (annotation?.comments) {
        const c = annotation.comments.find(c => c.id === _editingComment.id);
        if (c) { c.text = text; c.ts = nowTs(); }
      }
    } else {
      // add
      const data = await api.updateKbComment(kbRepo, kbPath, { text }, nowTs());
      if (!data.ok) throw new Error(data.error || 'failed');
      if (!annotation.comments) annotation.comments = [];
      annotation.comments.push({ id: data.id, text, ts: nowTs() });
    }
    closeKbCommentDialog();
    renderKbComments(annotation);
    document.dispatchEvent(new CustomEvent('kb:dirty', { detail: { msg: 'chore: update annotations' } }));
  } catch (e) {
    alert(`Save failed: ${e.message}`);
  }
}

// ── initKbComments / cleanupKbComments ─────────────────────────────────────
function bindKbCommentDialogEvents() {
  if (_kbDialogEventsBound) return;
  _kbDialogEventsBound = true;

  document.getElementById('kb-btn-comment-cancel')?.addEventListener('click', closeKbCommentDialog);
  document.getElementById('kb-btn-comment-save')?.addEventListener('click', saveKbComment);

  document.getElementById('kb-comment-dialog')?.addEventListener('keydown', e => {
    if (e.key === 'Escape') { e.stopPropagation(); closeKbCommentDialog(); }
  });

  document.getElementById('kb-comment-dialog-content')?.addEventListener('keydown', e => {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      e.preventDefault();
      saveKbComment();
    }
  });

  document.getElementById('kb-comment-dialog-content')?.addEventListener('paste', e => {
    e.preventDefault();
    const el = document.getElementById('kb-comment-dialog-content');
    pasteIntoCommentEditor(el, e.clipboardData);
  });

  document.querySelectorAll('#kb-comment-dialog .comment-tab-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      document.querySelectorAll('#kb-comment-dialog .comment-tab-btn').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      const isPreview = btn.dataset.tab === 'preview';
      const editorBox = document.getElementById('kb-comment-editor-box');
      const previewPane = document.getElementById('kb-comment-preview-pane');
      if (isPreview) {
        const text = prepareCommentMarkdown(
          document.getElementById('kb-comment-dialog-content').textContent
        );
        const inner = renderCommentMarkdown(text);
        previewPane.innerHTML = `<div class="comment-item-text">${inner}</div>`;
        await renderMermaidBlocks(previewPane);
        editorBox.style.display = 'none';
        previewPane.style.display = 'block';
      } else {
        editorBox.style.display = '';
        previewPane.style.display = 'none';
      }
    });
  });
}

export function cleanupKbComments() {
  for (const fn of _kbCommentsCleanups) fn();
  _kbCommentsCleanups.length = 0;
  if (_kbFloatNavObserver) {
    _kbFloatNavObserver.disconnect();
    _kbFloatNavObserver = null;
  }
  kbCommentsBodyEl()?.querySelector('.md-comments-bar')?.remove();
  const nav = kbFloatNavEl();
  if (nav) {
    nav.innerHTML = '';
    nav.style.display = 'none';
  }
  _kbCommentsRoot = null;
}

export function initKbComments(container) {
  if (!container) return;
  cleanupKbComments();
  _kbCommentsRoot = container;
  bindKbCommentDialogEvents();

  const addBtn = container.querySelector('.kb-btn-add-comment');
  if (addBtn) {
    const onAdd = () => openKbCommentDialog();
    addBtn.addEventListener('click', onAdd);
    _kbCommentsCleanups.push(() => addBtn.removeEventListener('click', onAdd));
  }
}

/** @deprecated use initKbComments(container) */
export function initKbCommentEvents() {
  const legacyBody = document.getElementById('kb-md-body');
  const root = legacyBody?.closest('.kb-reader') ?? legacyBody?.parentElement;
  if (root) initKbComments(root);
}
