import { useEffect, useSyncExternalStore } from 'react';
import { state } from '../host/state.ts';
import * as api from '../host/api.ts';
import { nowTs } from './utils.ts';
import { createModuleStore } from './module-store.ts';
import type { Annotation, LayerData, WriteResult } from './types.ts';

const commentCopy = {
  title: 'Delete comment',
  message: 'Delete this comment? This cannot be undone.',
  action: 'Delete',
};

const highlightCopy = {
  title: 'Remove highlight',
  message: 'This will remove the highlight and its saved comments. This cannot be undone.',
  action: 'Remove',
};

type Confirmation = typeof commentCopy & {
  trigger: HTMLElement | null;
  resolve: (result: boolean) => void;
};

const confirmationStore = createModuleStore<Confirmation | null>(null);
let dialogMounted = false;

function finishConfirm(result: boolean, restoreFocus = true) {
  const confirmation = confirmationStore.getSnapshot();
  if (!confirmation) return;
  confirmationStore.set(null);
  document.getElementById('comment-delete-dialog')?.classList.remove('open');
  if (restoreFocus && confirmation.trigger?.isConnected) {
    confirmation.trigger.focus({ preventScroll: true });
  }
  confirmation.resolve(result);
}

/** Kept for tests that still call it; the dialog wires itself when mounted. */
export function initCommentDeleteConfirm() {}

/** In-app confirm for Tauri WKWebView. */
function requestConfirmation(copy: typeof commentCopy, trigger?: HTMLElement): Promise<boolean> {
  if (!dialogMounted) return Promise.resolve(false);
  const current = confirmationStore.getSnapshot();
  const dialog = document.getElementById('comment-delete-dialog');
  const active = document.activeElement;
  const priorFocus = dialog?.contains(active) ? current?.trigger : active;
  finishConfirm(false, false);
  return new Promise<boolean>((resolve) => {
    confirmationStore.set({
      ...copy,
      trigger: trigger ?? (priorFocus instanceof HTMLElement ? priorFocus : null),
      resolve,
    });
    dialog?.classList.add('open');
    document.getElementById('btn-comment-delete-cancel')?.focus({ preventScroll: true });
  });
}

export function confirmDeleteComment(): Promise<boolean> {
  return requestConfirmation(commentCopy);
}

export function confirmRemoveHighlight(trigger?: HTMLElement): Promise<boolean> {
  return requestConfirmation(highlightCopy, trigger);
}

export async function removeNoteComment(
  c: { id: string },
  layer: string,
  entry: { _id?: string; id?: string },
) {
  const data = (await api.updateComments(entry._id || entry.id || '', layer, { id: c.id, text: '' }, nowTs())) as WriteResult;
  if (!data.ok) throw new Error(data.error || 'failed');
  const annotation = state.viewer.annotation as Annotation;
  const ld = (annotation[layer] || {}) as LayerData;
  ld.comments = (ld.comments || []).filter((x) => x.id !== c.id);
  if (!ld.comments.length) delete annotation[layer];
  else annotation[layer] = ld;
}

export function CommentDeleteDialog() {
  const confirmation = useSyncExternalStore(confirmationStore.subscribe, confirmationStore.getSnapshot);
  const copy = confirmation ?? commentCopy;

  useEffect(() => {
    dialogMounted = true;
    const onKey = (event: KeyboardEvent) => {
      if (!confirmationStore.getSnapshot()) return;
      if (event.key === 'Escape') {
        event.preventDefault();
        event.stopPropagation();
        finishConfirm(false);
      } else if (event.key === 'Tab' && !event.altKey && !event.ctrlKey && !event.metaKey) {
        const cancel = document.getElementById('btn-comment-delete-cancel');
        const confirm = document.getElementById('btn-comment-delete-ok');
        const active = document.activeElement;
        const target = active === cancel || (event.shiftKey && active !== confirm) ? confirm : cancel;
        event.preventDefault();
        target?.focus();
      }
    };
    window.addEventListener('keydown', onKey, true);
    return () => {
      window.removeEventListener('keydown', onKey, true);
      dialogMounted = false;
      finishConfirm(false);
    };
  }, []);

  return (
    <div
      id="comment-delete-dialog"
      className={confirmation ? 'open' : undefined}
      onClick={(e) => {
        if (e.target === e.currentTarget) finishConfirm(false);
      }}
    >
      <div
        id="comment-delete-dialog-box"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="comment-delete-dialog-title"
        aria-describedby="comment-delete-dialog-description"
      >
        <h3 id="comment-delete-dialog-title">{copy.title}</h3>
        <p id="comment-delete-dialog-description">{copy.message}</p>
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
            {copy.action}
          </button>
        </div>
      </div>
    </div>
  );
}
