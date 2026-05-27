import { state } from '../state.js'
import * as api from '../api.js'
import { reorderComments } from '../api.js'
import {
  ensureLayerComments,
  swapAdjacent,
  validateCommentIdsForReorder,
} from '../comment-reorder.js'
import { nowTs } from '../utils.js'
import { openSettleDialog } from './settle-dialog.js'
import { renderLinksBar } from './links-bar.js'
import { confirmDeleteComment, removeCorpusComment } from './comment-delete.js'
import {
  pasteIntoCommentEditor,
  prepareCommentMarkdown,
  renderCommentMarkdown,
} from '../comment-markdown.js'

// ── renderComments ─────────────────────────────────────────────────────────

let _floatNavObserver = null;

const _tip = () => document.getElementById('comment-preview-tip');

function _showTip(text, e) {
  const tip = _tip();
  if (!tip) return;
  const preview = text.length > 600 ? text.slice(0, 600) + '\n\n…' : text;
  const inner = renderCommentMarkdown(preview);
  tip.innerHTML = `<div class="comment-item-text">${inner}</div>`;
  tip.style.display = 'block';
  _moveTip(e);
}

function _moveTip(e) {
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

function _hideTip() {
  const tip = _tip();
  if (tip) tip.style.display = 'none';
}

function updateFloatNav(comments, layer, entry) {
  const nav = document.getElementById('comment-float-nav');
  if (!nav) return;
  nav.innerHTML = '';
  nav.style.display = 'none';
  if (_floatNavObserver) {
    _floatNavObserver.disconnect();
    _floatNavObserver = null;
  }
  if (!comments || comments.length === 0) return;

  comments.forEach((c, i) => {
    const btn = document.createElement('button');
    btn.className = 'comment-float-btn';
    btn.textContent = String(i + 1);
    btn.title = `编辑笔记 ${i + 1}`;
    btn.addEventListener('click', () => openCommentDialog(c, layer, entry, i + 1));
    btn.addEventListener('mouseenter', e => _showTip(c.text, e));
    btn.addEventListener('mousemove',  e => _moveTip(e));
    btn.addEventListener('mouseleave', () => _hideTip());
    nav.appendChild(btn);
  });

  const bar = document.getElementById('md-comments-bar');
  if (!bar) return;
  const mdBody = document.getElementById('md-body');
  _floatNavObserver = new IntersectionObserver(
    ([e]) => { nav.style.display = e.isIntersecting ? 'none' : 'flex'; },
    { root: mdBody, threshold: 0 }
  );
  _floatNavObserver.observe(bar);
}

export function renderComments(annotation, layer, entry) {
  const existing = document.getElementById('md-comments-bar');
  if (existing) existing.remove();
  if (!entry) { updateFloatNav([], null, null); return; }

  const layerData = (annotation && annotation[layer]) || {};
  const comments = layerData.comments || [];
  if (comments.length === 0) { updateFloatNav([], layer, entry); return; }

  const body = document.getElementById('md-body');
  const bar = document.createElement('div');
  bar.id = 'md-comments-bar';
  bar.className = 'md-comments-bar';

  const hdr = document.createElement('div');
  hdr.className = 'comment-bar-header';
  hdr.innerHTML = `<span>💬 笔记 · ${comments.length} 条</span>`;
  bar.appendChild(hdr);

  for (let i = 0; i < comments.length; i++) {
    bar.appendChild(buildCommentItem(comments[i], layer, entry, i + 1, comments));
  }

  body.insertBefore(bar, body.firstChild);
  updateFloatNav(comments, layer, entry);
}

async function moveCorpusComment(layer, entry, idx, delta) {
  const arr = ensureLayerComments(state.viewer.annotation, layer);
  if (!swapAdjacent(arr, idx, delta)) return;
  renderComments(state.viewer.annotation, layer, entry);
  const check = validateCommentIdsForReorder(arr);
  if (!check.ok) {
    alert(check.error);
    return;
  }
  try {
    const data = await reorderComments(entry.common_path, layer, check.ids);
    if (data?.ok !== true) throw new Error(data?.error || 'failed');
  } catch (e) {
    alert(`排序保存失败：${e.message}`);
  }
}

// ── buildCommentItem ───────────────────────────────────────────────────────

function buildCommentItem(c, layer, entry, noteIndex, allComments) {
  const item = document.createElement('div');
  item.className = 'comment-item';
  item.dataset.cid = c.id;

  const prefixEl = document.createElement('span');
  prefixEl.className = 'comment-item-prefix';
  prefixEl.textContent = `NOTE ${noteIndex}:`;

  const textEl = document.createElement('div');
  textEl.className = 'comment-item-text';
  if (typeof marked !== 'undefined') {
    textEl.innerHTML = renderCommentMarkdown(c.text);
  } else {
    textEl.textContent = c.text;
  }

  const tsEl = document.createElement('span');
  tsEl.className = 'comment-item-ts';
  if (c.ts && c.ts.length >= 12) {
    tsEl.textContent = `${c.ts.slice(0,4)}-${c.ts.slice(4,6)}-${c.ts.slice(6,8)} ${c.ts.slice(8,10)}:${c.ts.slice(10,12)}`;
  }

  const delX = document.createElement('button');
  delX.type = 'button';
  delX.className = 'comment-item-del-x';
  delX.title = '删除';
  delX.textContent = '×';
  delX.addEventListener('click', async () => {
    if (!(await confirmDeleteComment())) return;
    try {
      await removeCorpusComment(c, layer, entry);
      renderComments(state.viewer.annotation, layer, entry);
    } catch (e) { alert(`删除失败：${e.message}`); }
  });

  const actionsEl = document.createElement('div');
  actionsEl.className = 'comment-item-actions';

  const settleBtn = document.createElement('button');
  settleBtn.type = 'button';
  settleBtn.className = 'comment-item-action-btn';
  settleBtn.textContent = '⬆ 沉淀';
  settleBtn.title = '沉淀到知识仓库';
  settleBtn.addEventListener('click', () => openSettleDialog(c, layer, entry));

  const editBtn = document.createElement('button');
  editBtn.type = 'button';
  editBtn.className = 'comment-item-action-btn';
  editBtn.textContent = '编辑';
  editBtn.addEventListener('click', () => openCommentDialog(c, layer, entry, noteIndex));

  actionsEl.append(settleBtn, editBtn);

  const idx = noteIndex - 1;
  if (allComments && idx > 0) {
    const upBtn = document.createElement('button');
    upBtn.type = 'button';
    upBtn.className = 'comment-item-action-btn comment-item-order-btn';
    upBtn.title = '上移';
    upBtn.textContent = '↑';
    upBtn.addEventListener('click', () => moveCorpusComment(layer, entry, idx, -1));
    actionsEl.append(upBtn);
  }

  if (allComments && idx < allComments.length - 1) {
    const downBtn = document.createElement('button');
    downBtn.type = 'button';
    downBtn.className = 'comment-item-action-btn comment-item-order-btn';
    downBtn.title = '下移';
    downBtn.textContent = '↓';
    downBtn.addEventListener('click', () => moveCorpusComment(layer, entry, idx, 1));
    actionsEl.append(downBtn);
  }

  item.append(delX, prefixEl, textEl, tsEl, actionsEl);
  return item;
}

// ── Comment dialog ─────────────────────────────────────────────────────────

let _commentEditCtx = null;
let _draftKey = null;      // common_path used as cache key for current draft
let _draftSaveTimer = null;
let _previewModeText = null; // cached text when editorBox is hidden (innerText breaks on hidden elements)

function _scheduleDraftSave() {
  if (!_draftKey) return;
  clearTimeout(_draftSaveTimer);
  _draftSaveTimer = setTimeout(() => {
    const text = prepareCommentMarkdown(document.getElementById('comment-dialog-content').innerText);
    api.saveDraft(_draftKey, text).catch(() => {});
  }, 800);
}

function _clearDraft() {
  if (!_draftKey) return;
  clearTimeout(_draftSaveTimer);
  api.saveDraft(_draftKey, '').catch(() => {});
  _draftKey = null;
}

export async function openCommentDialog(editComment, layer, entry, noteIndex) {
  if (!state.viewer.entry && !entry) return;
  const titleEl = document.getElementById('comment-dialog-title');
  const content = document.getElementById('comment-dialog-content');
  if (editComment) {
    _commentEditCtx = { c: editComment, layer, entry, noteIndex };
    _draftKey = null;
    titleEl.textContent = '💬 编辑笔记';
    content.innerText = editComment.text;
  } else {
    _commentEditCtx = { noteIndex };
    const commonPath = (entry || state.viewer.entry).common_path;
    _draftKey = commonPath;
    titleEl.textContent = '💬 添加笔记';
    content.innerText = '';
    // Load cached draft
    try {
      const draft = await api.getDraft(commonPath);
      if (draft.content) content.innerText = draft.content;
    } catch (e) { /* ignore */ }
  }
  document.getElementById('comment-dialog').classList.add('open');
  _resetDialogTabs();
  requestAnimationFrame(() => {
    content.focus();
    const range = document.createRange();
    const sel = window.getSelection();
    range.selectNodeContents(content);
    range.collapse(false);
    sel.removeAllRanges();
    sel.addRange(range);
  });
}

export function closeCommentDialog() {
  document.getElementById('comment-dialog').classList.remove('open');
  _commentEditCtx = null;
  _clearDraft();
}

export async function saveComment() {
  if (!_commentEditCtx) return;
  const content = document.getElementById('comment-dialog-content');
  const text = prepareCommentMarkdown(
    (_previewModeText !== null ? _previewModeText : content.innerText).trim()
  );
  if (!text) return;
  const saveBtn = document.getElementById('btn-comment-save');
  saveBtn.disabled = true;
  saveBtn.textContent = '保存中…';
  const ts = nowTs();
  try {
    if (_commentEditCtx.c) {
      const { c, layer, entry } = _commentEditCtx;
      if (text === c.text) { closeCommentDialog(); return; }
      const data = await api.updateComments(entry.common_path, layer, { id: c.id, text }, ts);
      if (data.ok) {
        c.text = text;
        const ld = state.viewer.annotation[layer] || {};
        const idx = (ld.comments || []).findIndex(x => x.id === c.id);
        if (idx >= 0) ld.comments[idx].text = text;
        closeCommentDialog();
        renderComments(state.viewer.annotation, layer, entry);
      } else { alert(`保存失败：${data.error}`); }
    } else {
      if (!state.viewer.entry) return;
      const data = await api.updateComments(state.viewer.entry.common_path, state.viewer.layer, { text }, ts);
      if (data.ok) {
        if (!state.viewer.annotation[state.viewer.layer]) state.viewer.annotation[state.viewer.layer] = {};
        if (!state.viewer.annotation[state.viewer.layer].comments) state.viewer.annotation[state.viewer.layer].comments = [];
        state.viewer.annotation[state.viewer.layer].comments.push({ id: data.id, text, ts });
        _clearDraft();
        closeCommentDialog();
        renderComments(state.viewer.annotation, state.viewer.layer, state.viewer.entry);
      } else { alert(`添加失败：${data.error}`); }
    }
  } catch (e) { alert(`保存失败：${e.message}`); }
  finally { saveBtn.disabled = false; saveBtn.textContent = '保存'; }
}

// ── Event listeners ────────────────────────────────────────────────────────

// Tab switch: edit / preview
document.querySelectorAll('.comment-tab-btn').forEach(btn => {
  btn.addEventListener('click', () => {
    document.querySelectorAll('.comment-tab-btn').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    const isPreview = btn.dataset.tab === 'preview';
    const editorBox = document.getElementById('comment-editor-box');
    const previewPane = document.getElementById('comment-preview-pane');
    if (isPreview) {
      _previewModeText = prepareCommentMarkdown(
        document.getElementById('comment-dialog-content').innerText
      );
      const inner = renderCommentMarkdown(_previewModeText);
      previewPane.innerHTML = `<div class="comment-item-text">${inner}</div>`;
      editorBox.style.display = 'none';
      previewPane.style.display = 'block';
    } else {
      _previewModeText = null;
      editorBox.style.display = '';
      previewPane.style.display = 'none';
    }
  });
});

// Reset to edit tab whenever the dialog opens
function _resetDialogTabs() {
  _previewModeText = null;
  document.querySelectorAll('.comment-tab-btn').forEach(b => {
    b.classList.toggle('active', b.dataset.tab === 'edit');
  });
  document.getElementById('comment-editor-box').style.display = '';
  document.getElementById('comment-preview-pane').style.display = 'none';
}

document.getElementById('btn-add-comment').addEventListener('click', () => {
  const layerData = (state.viewer.annotation && state.viewer.annotation[state.viewer.layer]) || {};
  const nextIdx = (layerData.comments || []).length + 1;
  openCommentDialog(null, null, null, nextIdx);
});
document.getElementById('btn-comment-cancel').addEventListener('click', closeCommentDialog);
document.getElementById('btn-comment-save').addEventListener('click', saveComment);
// ESC on the dialog container — catches all focus states (edit tab, preview tab, buttons)
document.getElementById('comment-dialog').addEventListener('keydown', e => {
  if (e.key === 'Escape') { e.stopPropagation(); closeCommentDialog(); }
});
document.getElementById('comment-dialog-content').addEventListener('keydown', e => {
  if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) { e.preventDefault(); saveComment(); }
});
document.getElementById('comment-dialog-content').addEventListener('input', () => {
  _scheduleDraftSave();
});
document.getElementById('comment-dialog-content').addEventListener('paste', e => {
  e.preventDefault();
  const el = document.getElementById('comment-dialog-content');
  pasteIntoCommentEditor(el, e.clipboardData);
  // Scroll cursor into view after insert (contenteditable doesn't do this automatically)
  const sel = window.getSelection();
  if (sel && sel.rangeCount > 0) {
    const range = sel.getRangeAt(0);
    const rect = range.getBoundingClientRect();
    const elRect = el.getBoundingClientRect();
    if (rect.bottom > elRect.bottom) {
      el.scrollTop += rect.bottom - elRect.bottom + 8;
    }
  }
});
// Close only when *both* mousedown and click land on the overlay backdrop,
// so dragging from inside the dialog box to outside won't dismiss it.
let _mouseDownOnOverlay = false;
document.getElementById('comment-dialog').addEventListener('mousedown', e => {
  _mouseDownOnOverlay = (e.target === document.getElementById('comment-dialog'));
});
document.getElementById('comment-dialog').addEventListener('click', e => {
  if (e.target === document.getElementById('comment-dialog') && _mouseDownOnOverlay) {
    closeCommentDialog();
  }
});

// Handle successful settle: remove comment from state + re-render links bar
document.addEventListener('settle:done', ({ detail }) => {
  const { commentId, layer, entry, url } = detail;
  const ld = state.viewer.annotation[layer] || {};
  ld.comments = (ld.comments || []).filter(x => x.id !== commentId);
  if (!ld.comments.length) delete state.viewer.annotation[layer];
  else state.viewer.annotation[layer] = ld;
  if (!state.viewer.annotation.links) state.viewer.annotation.links = [];
  state.viewer.annotation.links.push({ url });
  // Sync entry.links so renderLinksBar reads the updated list
  entry.links = state.viewer.annotation.links;
  renderComments(state.viewer.annotation, layer, entry);
  renderLinksBar(entry);
});
