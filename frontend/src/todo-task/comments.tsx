// @ts-nocheck
import { renderToHtml } from '../island.ts';
import { formatRelativeTime } from './format.ts';
import {
  addPlanComment,
  deletePlanComment,
  listPlanComments,
  updatePlanComment,
} from './host.ts';

const COMMENTS_EMPTY_MSG = 'No process notes';

export function renderCommentsSection(ui) {
  return renderToHtml(<CommentsSection ui={ui} />);
}

export function CommentsSection({ ui }) {
  const items = ui.comments ?? [];
  const editId = ui.commentEditId || '';
  const confirmId = ui.commentDeleteConfirm || '';
  return (
    <section className="todo-task-comments-section" aria-label="Process notes">
      <div className="todo-task-comments-header">
        <div className="todo-task-comments-heading">
          <h3 className="todo-task-comments-title">Process notes</h3>
          {items.length ? (
            <span className="todo-task-comments-count" aria-label={`${items.length} process notes`}>
              {items.length}
            </span>
          ) : null}
        </div>
        {!items.length && !ui.commentsError ? (
          <span className="todo-task-comments-empty">{COMMENTS_EMPTY_MSG}</span>
        ) : null}
      </div>
      {ui.commentsError ? (
        <p className="todo-task-comments-error" role="alert">
          {ui.commentsError}
        </p>
      ) : null}
      {items.length ? (
        <ul className="todo-task-comment-list" role="list">
          {items.map((entry) => {
            const added = formatRelativeTime(entry.created_at);
            const meta = added ? `Added ${added}` : 'Process note';
            if (editId === entry.id) {
              return (
                <li
                  key={entry.id}
                  className="todo-task-comment-item todo-task-comment-item--editing"
                  data-comment-id={entry.id}
                >
                  <textarea
                    className="todo-task-comment-edit-area"
                    data-comment-edit-input=""
                    spellCheck={true}
                    disabled={ui.disabled}
                    defaultValue={entry.body ?? ''}
                  />
                  <div className="todo-task-comment-item-actions">
                    <button
                      type="button"
                      className="md-header-btn"
                      data-action="cancel-comment-edit"
                      disabled={ui.disabled}
                    >
                      Cancel
                    </button>
                    <button
                      type="button"
                      className="md-header-btn primary"
                      data-action="save-comment"
                      data-comment-id={entry.id}
                      disabled={ui.disabled}
                    >
                      Save
                    </button>
                  </div>
                </li>
              );
            }
            return (
              <li key={entry.id} className="todo-task-comment-item" data-comment-id={entry.id}>
                <div className="todo-task-comment-main">
                  <p className="todo-task-comment-body">{entry.body ?? ''}</p>
                  <span className="todo-task-comment-meta">{meta}</span>
                </div>
                <div className="todo-task-comment-item-actions">
                  <button
                    type="button"
                    className="md-header-btn"
                    data-action="edit-comment"
                    data-comment-id={entry.id}
                    disabled={ui.disabled}
                  >
                    Edit
                  </button>
                  <button
                    type="button"
                    className="todo-task-comment-delete"
                    data-action="delete-comment"
                    data-comment-id={entry.id}
                    aria-label="Delete process note"
                    title="Delete"
                    disabled={ui.disabled}
                  >
                    Delete
                  </button>
                </div>
              </li>
            );
          })}
        </ul>
      ) : null}
      <div className="todo-task-comments-composer">
        <label className="todo-task-comments-composer-label" htmlFor="todo-task-comment-input">
          New process note
        </label>
        <textarea
          id="todo-task-comment-input"
          className="todo-task-comment-input"
          data-comment-input=""
          rows={3}
          placeholder="Add a process note…"
          spellCheck={true}
          disabled={ui.disabled}
        />
        <div className="todo-task-comments-composer-actions">
          <button
            type="button"
            className="md-header-btn primary"
            data-action="add-comment"
            disabled={ui.disabled}
          >
            Add note
          </button>
        </div>
      </div>
      {confirmId ? (
        <div
          className="todo-task-comment-delete-confirm"
          data-comment-delete-confirm=""
          role="dialog"
          aria-modal="true"
          aria-label="Delete process note confirmation"
        >
          <div
            className="todo-task-comment-delete-confirm-backdrop"
            data-action="cancel-delete-comment"
          />
          <div className="todo-task-comment-delete-confirm-panel">
            <h4 className="todo-task-comment-delete-confirm-title">Delete process note</h4>
            <p className="todo-task-comment-delete-confirm-body">
              Delete this process note? This cannot be undone.
            </p>
            <div className="todo-task-comment-delete-confirm-actions">
              <button
                type="button"
                className="md-header-btn"
                data-action="cancel-delete-comment"
                disabled={ui.disabled}
              >
                Cancel
              </button>
              <button
                type="button"
                className="md-header-btn todo-task-btn-danger"
                data-action="confirm-delete-comment"
                data-comment-id={confirmId}
                disabled={ui.disabled}
              >
                Confirm delete
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </section>
  );
}

export function createCommentsOwner(ctx) {
  let comments = [];
  let commentsError = '';
  let commentEditId = '';
  let commentDeleteConfirm = '';

  function resetForSelectionChange() {
    comments = [];
    commentsError = '';
    commentEditId = '';
    commentDeleteConfirm = '';
  }

  async function loadForSelected() {
    const selectedMasterId = ctx.getSelectedMasterId();
    if (!selectedMasterId) {
      comments = [];
      commentsError = '';
      return;
    }
    const masterId = selectedMasterId;
    try {
      const entries = await listPlanComments({ masterTaskId: masterId });
      if (ctx.isDisposed() || ctx.getSelectedMasterId() !== masterId) return;
      comments = entries;
      commentsError = '';
    } catch (err) {
      if (ctx.isDisposed() || ctx.getSelectedMasterId() !== masterId) return;
      comments = [];
      commentsError = err?.message || 'Failed to load process notes';
    }
  }

  async function addFromComposer() {
    const selectedMasterId = ctx.getSelectedMasterId();
    if (!selectedMasterId || ctx.isBusy()) return;
    const input = ctx.getContainer().querySelector('[data-comment-input]');
    const body = input instanceof HTMLTextAreaElement ? input.value : '';
    const masterTaskId = selectedMasterId;
    ctx.setBusy(true);
    commentsError = '';
    ctx.paint();
    try {
      await addPlanComment({ masterTaskId, body });
      if (ctx.isDisposed() || ctx.getSelectedMasterId() !== masterTaskId) return;
      commentsError = '';
      commentEditId = '';
      await loadForSelected();
    } catch (err) {
      if (ctx.isDisposed() || ctx.getSelectedMasterId() !== masterTaskId) return;
      commentsError = err?.message || 'Failed to add process note';
    } finally {
      ctx.setBusy(false);
      if (!ctx.isDisposed()) ctx.paint();
    }
  }

  function beginEdit(commentId) {
    if (!ctx.getSelectedMasterId() || ctx.isBusy() || !commentId) return;
    commentEditId = commentId;
    commentDeleteConfirm = '';
    commentsError = '';
    ctx.paint();
  }

  function cancelEdit() {
    commentEditId = '';
    ctx.paint();
  }

  async function saveEdit(commentId) {
    const selectedMasterId = ctx.getSelectedMasterId();
    if (!selectedMasterId || ctx.isBusy() || !commentId) return;
    const editInput = ctx.getContainer().querySelector('[data-comment-edit-input]');
    const body = editInput instanceof HTMLTextAreaElement ? editInput.value : '';
    const masterTaskId = selectedMasterId;
    ctx.setBusy(true);
    commentsError = '';
    ctx.paint();
    try {
      await updatePlanComment({ masterTaskId, commentId, body });
      if (ctx.isDisposed() || ctx.getSelectedMasterId() !== masterTaskId) return;
      commentEditId = '';
      commentsError = '';
      await loadForSelected();
    } catch (err) {
      if (ctx.isDisposed() || ctx.getSelectedMasterId() !== masterTaskId) return;
      commentsError = err?.message || 'Failed to update process note';
    } finally {
      ctx.setBusy(false);
      if (!ctx.isDisposed()) ctx.paint();
    }
  }

  function openDeleteConfirm(commentId) {
    if (!ctx.getSelectedMasterId() || ctx.isBusy() || !commentId) return;
    commentDeleteConfirm = commentId;
    commentEditId = '';
    commentsError = '';
    ctx.paint();
  }

  function cancelDeleteConfirm() {
    commentDeleteConfirm = '';
    ctx.paint();
  }

  async function confirmDelete(commentId) {
    const selectedMasterId = ctx.getSelectedMasterId();
    if (!selectedMasterId || ctx.isBusy() || !commentId) return;
    const masterTaskId = selectedMasterId;
    ctx.setBusy(true);
    commentsError = '';
    ctx.paint();
    try {
      await deletePlanComment({ masterTaskId, commentId });
      if (ctx.isDisposed() || ctx.getSelectedMasterId() !== masterTaskId) return;
      commentDeleteConfirm = '';
      commentsError = '';
      await loadForSelected();
    } catch (err) {
      if (ctx.isDisposed() || ctx.getSelectedMasterId() !== masterTaskId) return;
      commentDeleteConfirm = '';
      commentsError = err?.message || 'Failed to delete process note';
    } finally {
      ctx.setBusy(false);
      if (!ctx.isDisposed()) ctx.paint();
    }
  }

  return {
    uiSlice() {
      return { comments, commentsError, commentEditId, commentDeleteConfirm };
    },
    get editId() {
      return commentEditId;
    },
    get deleteConfirm() {
      return commentDeleteConfirm;
    },
    resetForSelectionChange,
    clearOnListError() {
      comments = [];
      commentsError = '';
      commentEditId = '';
      commentDeleteConfirm = '';
    },
    loadForSelected,
    handleClick(event, action, actionEl) {
      if (action === 'add-comment') {
        void addFromComposer();
        return true;
      }
      if (action === 'edit-comment') {
        const commentId = actionEl?.dataset.commentId;
        if (!commentId) return true;
        beginEdit(commentId);
        return true;
      }
      if (action === 'save-comment') {
        const commentId = actionEl?.dataset.commentId || commentEditId;
        if (!commentId) return true;
        void saveEdit(commentId);
        return true;
      }
      if (action === 'cancel-comment-edit') {
        cancelEdit();
        return true;
      }
      if (action === 'delete-comment') {
        const commentId = actionEl?.dataset.commentId;
        if (!commentId) return true;
        openDeleteConfirm(commentId);
        return true;
      }
      if (action === 'confirm-delete-comment') {
        event.stopPropagation();
        const commentId = actionEl?.dataset.commentId || commentDeleteConfirm;
        if (!commentId) return true;
        void confirmDelete(commentId);
        return true;
      }
      if (action === 'cancel-delete-comment') {
        event.stopPropagation();
        cancelDeleteConfirm();
        return true;
      }
      return false;
    },
    handleKeydown(event) {
      if (event.key !== 'Escape') return false;
      if (commentDeleteConfirm) {
        event.preventDefault();
        cancelDeleteConfirm();
        return true;
      }
      if (commentEditId) {
        event.preventDefault();
        if (ctx.isBusy()) return true;
        cancelEdit();
        return true;
      }
      return false;
    },
  };
}
