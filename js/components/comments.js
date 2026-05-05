import { state } from '../state.js'
import * as api from '../api.js'
import { nowTs } from '../utils.js'

// ── renderComments ─────────────────────────────────────────────────────────

export function renderComments(annotation, layer, entry) {
  const existing = document.getElementById('md-comments-bar');
  if (existing) existing.remove();
  if (!entry) return;

  const layerData = (annotation && annotation[layer]) || {};
  const comments = layerData.comments || [];
  if (comments.length === 0) return;

  const body = document.getElementById('md-body');
  const bar = document.createElement('div');
  bar.id = 'md-comments-bar';

  const hdr = document.createElement('div');
  hdr.className = 'comment-bar-header';
  hdr.innerHTML = `<span>💬 笔记 · ${comments.length} 条</span>`;
  bar.appendChild(hdr);

  for (let i = 0; i < comments.length; i++) {
    bar.appendChild(buildCommentItem(comments[i], layer, entry, i + 1));
  }

  body.insertBefore(bar, body.firstChild);
}

// ── buildCommentItem ───────────────────────────────────────────────────────

function buildCommentItem(c, layer, entry, noteIndex) {
  const item = document.createElement('div');
  item.className = 'comment-item';
  item.dataset.cid = c.id;

  const prefixEl = document.createElement('span');
  prefixEl.className = 'comment-item-prefix';
  prefixEl.textContent = `NOTE ${noteIndex}:`;

  const textEl = document.createElement('div');
  textEl.className = 'comment-item-text';
  if (typeof marked !== 'undefined') {
    textEl.innerHTML = marked.parse(c.text);
  } else {
    textEl.textContent = c.text;
  }

  const tsEl = document.createElement('span');
  tsEl.className = 'comment-item-ts';
  if (c.ts && c.ts.length >= 12) {
    tsEl.textContent = `${c.ts.slice(0,4)}-${c.ts.slice(4,6)}-${c.ts.slice(6,8)} ${c.ts.slice(8,10)}:${c.ts.slice(10,12)}`;
  }

  const delX = document.createElement('button');
  delX.className = 'comment-item-del-x';
  delX.title = '删除';
  delX.textContent = '×';
  delX.addEventListener('click', async () => {
    if (!confirm('删除此笔记？')) return;
    try {
      const data = await api.updateComments(entry.common_path, layer, { id: c.id, text: '' }, nowTs());
      if (data.ok) {
        const ld = state.viewer.annotation[layer] || {};
        ld.comments = (ld.comments || []).filter(x => x.id !== c.id);
        if (!ld.comments.length) delete state.viewer.annotation[layer];
        else state.viewer.annotation[layer] = ld;
        renderComments(state.viewer.annotation, layer, entry);
      }
    } catch (e) { alert(`删除失败：${e.message}`); }
  });

  const editBtn = document.createElement('button');
  editBtn.className = 'comment-item-edit';
  editBtn.textContent = '编辑';
  editBtn.addEventListener('click', () => openCommentDialog(c, layer, entry, noteIndex));

  item.append(delX, prefixEl, textEl, tsEl, editBtn);
  return item;
}

// ── Comment dialog ─────────────────────────────────────────────────────────

let _commentEditCtx = null;

export function openCommentDialog(editComment, layer, entry, noteIndex) {
  if (!state.viewer.entry && !entry) return;
  const titleEl = document.getElementById('comment-dialog-title');
  const content = document.getElementById('comment-dialog-content');
  if (editComment) {
    _commentEditCtx = { c: editComment, layer, entry, noteIndex };
    titleEl.textContent = '💬 编辑笔记';
    content.innerText = editComment.text;
  } else {
    _commentEditCtx = { noteIndex };
    titleEl.textContent = '💬 添加笔记';
    content.innerText = '';
  }
  document.getElementById('comment-dialog').classList.add('open');
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
}

export async function saveComment() {
  if (!_commentEditCtx) return;
  const content = document.getElementById('comment-dialog-content');
  const text = content.innerText.trim();
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
        closeCommentDialog();
        renderComments(state.viewer.annotation, state.viewer.layer, state.viewer.entry);
      } else { alert(`添加失败：${data.error}`); }
    }
  } catch (e) { alert(`保存失败：${e.message}`); }
  finally { saveBtn.disabled = false; saveBtn.textContent = '保存'; }
}

// ── Event listeners ────────────────────────────────────────────────────────

document.getElementById('btn-add-comment').addEventListener('click', () => {
  const layerData = (state.viewer.annotation && state.viewer.annotation[state.viewer.layer]) || {};
  const nextIdx = (layerData.comments || []).length + 1;
  openCommentDialog(null, null, null, nextIdx);
});
document.getElementById('btn-comment-cancel').addEventListener('click', closeCommentDialog);
document.getElementById('btn-comment-save').addEventListener('click', saveComment);
document.getElementById('comment-dialog-content').addEventListener('keydown', e => {
  if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) { e.preventDefault(); saveComment(); }
  if (e.key === 'Escape') closeCommentDialog();
});
document.getElementById('comment-dialog').addEventListener('click', e => {
  if (e.target === document.getElementById('comment-dialog')) closeCommentDialog();
});
