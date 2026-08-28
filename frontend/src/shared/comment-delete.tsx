import { useEffect, useSyncExternalStore } from 'react';
import { state } from '../host/state.ts';
import * as api from '../host/api.ts';
import { nowTs } from './utils.ts';
import { createModuleStore } from './module-store.ts';
import type { Annotation, CommentLike, LayerData, WriteResult } from './types.ts';

const openStore = createModuleStore(false);

let pendingResolve: ((result: boolean) => void) | null = null;
let dialogMounted = false;

function finishConfirm(result: boolean) {
  openStore.set(false);
  document.getElementById('comment-delete-dialog')?.classList.remove('open');
  const resolve = pendingResolve;
  pendingResolve = null;
  if (resolve) resolve(result);
}

/** Kept for tests that still call it; the dialog wires itself when mounted. */
export function initCommentDeleteConfirm() {}

/** In-app confirm (replaces window.confirm for Tauri WKWebView). */
export function confirmDeleteComment() {
  if (!dialogMounted) return Promise.resolve(false);
  if (pendingResolve) finishConfirm(false);
  return new Promise<boolean>((resolve) => {
    pendingResolve = resolve;
    openStore.set(true);
    document.getElementById('comment-delete-dialog')?.classList.add('open');
  });
}

export async function removeKbComment(comment: { id: string }) {
  const { kbRepo, kbPath, annotation } = state.viewer;
  const data = (await api.updateKbComment(kbRepo, kbPath, { id: comment.id, text: '' }, nowTs())) as WriteResult;
  if (!data.ok) throw new Error(data.error || 'failed');
  const ann = annotation as Annotation;
  if (Array.isArray(ann.comments)) {
    ann.comments = (ann.comments as CommentLike[]).filter((c) => c.id !== comment.id);
  }
}

export async function removeCorpusComment(
  c: { id: string },
  layer: string,
  entry: { common_path: string },
) {
  const data = (await api.updateComments(entry.common_path, layer, { id: c.id, text: '' }, nowTs())) as WriteResult;
  if (!data.ok) throw new Error(data.error || 'failed');
  const annotation = state.viewer.annotation as Annotation;
  const ld = (annotation[layer] || {}) as LayerData;
  ld.comments = (ld.comments || []).filter((x) => x.id !== c.id);
  if (!ld.comments.length) delete annotation[layer];
  else annotation[layer] = ld;
}

export function CommentDeleteDialog() {
  const open = useSyncExternalStore(openStore.subscribe, openStore.getSnapshot);

  useEffect(() => {
    dialogMounted = true;
    return () => {
      dialogMounted = false;
    };
  }, []);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      e.stopPropagation();
      finishConfirm(false);
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [open]);

  return (
    <div
      id="comment-delete-dialog"
      className={open ? 'open' : undefined}
      onClick={(e) => {
        if (e.target === e.currentTarget) finishConfirm(false);
      }}
    >
      <div id="comment-delete-dialog-box">
        <h3>Delete comment</h3>
        <p>Delete this comment? This cannot be undone.</p>
        <div id="comment-delete-dialog-actions">
          <button
            type="button"
            id="btn-comment-delete-cancel"
            className="md-header-btn"
            onClick={(e) => {
              e.stopPropagation();
              finishConfirm(false);
            }}
          >
            Cancel
          </button>
          <button
            type="button"
            id="btn-comment-delete-ok"
            className="md-header-btn comment-delete-ok-btn"
            onClick={(e) => {
              e.stopPropagation();
              finishConfirm(true);
            }}
          >
            Delete
          </button>
        </div>
      </div>
    </div>
  );
}
