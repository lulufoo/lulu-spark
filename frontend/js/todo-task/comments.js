import { escHtml } from '../utils.js';
import { formatRelativeTime } from './format.js';
import {
  addPlanComment,
  deletePlanComment,
  listPlanComments,
  updatePlanComment,
} from './host.js';

const COMMENTS_EMPTY_MSG = 'No process notes';

export function renderCommentsSection(ui) {
  const disabledAttr = ui.disabled ? ' disabled' : '';
  const items = ui.comments ?? [];
  const editId = ui.commentEditId || '';
  const confirmId = ui.commentDeleteConfirm || '';
  const countHtml = items.length
    ? `<span class="todo-task-comments-count" aria-label="${items.length} process notes">${items.length}</span>`
    : '';
  const emptyHtml =
    !items.length && !ui.commentsError
      ? `<span class="todo-task-comments-empty">${escHtml(COMMENTS_EMPTY_MSG)}</span>`
      : '';
  const listHtml = items.length
    ? `<ul class="todo-task-comment-list" role="list">${items
        .map((entry) => {
          const added = formatRelativeTime(entry.created_at);
          const meta = added ? `Added ${added}` : 'Process note';
          if (editId === entry.id) {
            return `
        <li class="todo-task-comment-item todo-task-comment-item--editing" data-comment-id="${escHtml(entry.id)}">
          <textarea
            class="todo-task-comment-edit-area"
            data-comment-edit-input
            spellcheck="true"
            ${disabledAttr}
          >${escHtml(entry.body ?? '')}</textarea>
          <div class="todo-task-comment-item-actions">
            <button type="button" class="md-header-btn" data-action="cancel-comment-edit"${disabledAttr}>Cancel</button>
            <button
              type="button"
              class="md-header-btn primary"
              data-action="save-comment"
              data-comment-id="${escHtml(entry.id)}"
              ${disabledAttr}
            >Save</button>
          </div>
        </li>`;
          }
          return `
        <li class="todo-task-comment-item" data-comment-id="${escHtml(entry.id)}">
          <div class="todo-task-comment-main">
            <p class="todo-task-comment-body">${escHtml(entry.body ?? '')}</p>
            <span class="todo-task-comment-meta">${escHtml(meta)}</span>
          </div>
          <div class="todo-task-comment-item-actions">
            <button
              type="button"
              class="md-header-btn"
              data-action="edit-comment"
              data-comment-id="${escHtml(entry.id)}"
              ${disabledAttr}
            >Edit</button>
            <button
              type="button"
              class="todo-task-comment-delete"
              data-action="delete-comment"
              data-comment-id="${escHtml(entry.id)}"
              aria-label="Delete process note"
              title="Delete"
              ${disabledAttr}
            >Delete</button>
          </div>
        </li>`;
        })
        .join('')}</ul>`
    : '';
  const errHtml = ui.commentsError
    ? `<p class="todo-task-comments-error" role="alert">${escHtml(ui.commentsError)}</p>`
    : '';
  const confirmHtml = confirmId
    ? `
      <div
        class="todo-task-comment-delete-confirm"
        data-comment-delete-confirm
        role="dialog"
        aria-modal="true"
        aria-label="Delete process note confirmation"
      >
        <div class="todo-task-comment-delete-confirm-backdrop" data-action="cancel-delete-comment"></div>
        <div class="todo-task-comment-delete-confirm-panel">
          <h4 class="todo-task-comment-delete-confirm-title">Delete process note</h4>
          <p class="todo-task-comment-delete-confirm-body">Delete this process note? This cannot be undone.</p>
          <div class="todo-task-comment-delete-confirm-actions">
            <button
              type="button"
              class="md-header-btn"
              data-action="cancel-delete-comment"
              ${disabledAttr}
            >Cancel</button>
            <button
              type="button"
              class="md-header-btn todo-task-btn-danger"
              data-action="confirm-delete-comment"
              data-comment-id="${escHtml(confirmId)}"
              ${disabledAttr}
            >Confirm delete</button>
          </div>
        </div>
      </div>`
    : '';
  return `
    <section class="todo-task-comments-section" aria-label="Process notes">
      <div class="todo-task-comments-header">
        <div class="todo-task-comments-heading">
          <h3 class="todo-task-comments-title">Process notes</h3>
          ${countHtml}
        </div>
        ${emptyHtml}
      </div>
      ${errHtml}
      ${listHtml}
      <div class="todo-task-comments-composer">
        <label class="todo-task-comments-composer-label" for="todo-task-comment-input">New process note</label>
        <textarea
          id="todo-task-comment-input"
          class="todo-task-comment-input"
          data-comment-input
          rows="3"
          placeholder="Add a process note…"
          spellcheck="true"
          ${disabledAttr}
        ></textarea>
        <div class="todo-task-comments-composer-actions">
          <button type="button" class="md-header-btn primary" data-action="add-comment"${disabledAttr}>Add note</button>
        </div>
      </div>
      ${confirmHtml}
    </section>
  `;
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
