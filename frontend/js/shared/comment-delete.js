import { state } from '../host/state.js'
import * as api from '../host/api.js'
import { nowTs } from './utils.js'

let _pendingResolve = null;
let _eventsInited = false;

function getDialog() {
  return document.getElementById('comment-delete-dialog');
}

function finishConfirm(result) {
  const dialog = getDialog();
  if (dialog) dialog.classList.remove('open');
  const resolve = _pendingResolve;
  _pendingResolve = null;
  if (resolve) resolve(result);
}

export function initCommentDeleteConfirm() {
  if (_eventsInited) return;

  const dialog = getDialog();
  const cancelBtn = document.getElementById('btn-comment-delete-cancel');
  const okBtn = document.getElementById('btn-comment-delete-ok');
  if (!dialog || !cancelBtn || !okBtn) return;

  _eventsInited = true;

  cancelBtn.addEventListener('click', e => {
    e.stopPropagation();
    finishConfirm(false);
  });
  okBtn.addEventListener('click', e => {
    e.stopPropagation();
    finishConfirm(true);
  });
  dialog.addEventListener('click', e => {
    if (e.target === dialog) finishConfirm(false);
  });
  document.addEventListener('keydown', e => {
    if (e.key !== 'Escape') return;
    if (!dialog.classList.contains('open')) return;
    e.stopPropagation();
    finishConfirm(false);
  });
}

/** In-app confirm (replaces window.confirm for Tauri WKWebView). */
export function confirmDeleteComment() {
  initCommentDeleteConfirm();
  const dialog = getDialog();
  if (!dialog) return Promise.resolve(false);
  if (_pendingResolve) finishConfirm(false);
  return new Promise(resolve => {
    _pendingResolve = resolve;
    dialog.classList.add('open');
  });
}

export async function removeKbComment(comment) {
  const { kbRepo, kbPath, annotation } = state.viewer;
  const data = await api.updateKbComment(kbRepo, kbPath, { id: comment.id, text: '' }, nowTs());
  if (!data.ok) throw new Error(data.error || 'failed');
  if (annotation?.comments) {
    annotation.comments = annotation.comments.filter(c => c.id !== comment.id);
  }
}

export async function removeCorpusComment(c, layer, entry) {
  const data = await api.updateComments(entry.common_path, layer, { id: c.id, text: '' }, nowTs());
  if (!data.ok) throw new Error(data.error || 'failed');
  const ld = state.viewer.annotation[layer] || {};
  ld.comments = (ld.comments || []).filter(x => x.id !== c.id);
  if (!ld.comments.length) delete state.viewer.annotation[layer];
  else state.viewer.annotation[layer] = ld;
}
