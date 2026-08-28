import {
  addPlanComment,
  deletePlanComment,
  listPlanComments,
  updatePlanComment,
} from '../state/host.ts';
import { elementValue, errMessage, type TodoComment, type TodoPageCtx } from '../state/types.ts';

export function createCommentsOwner(ctx: TodoPageCtx) {
  let comments: TodoComment[] = [];
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
      commentsError = errMessage(err, 'Failed to load process notes');
    }
  }

  async function addFromComposer() {
    const selectedMasterId = ctx.getSelectedMasterId();
    if (!selectedMasterId || ctx.isBusy()) return;
    const input = ctx.getContainer().querySelector('[data-comment-input]');
    const body = elementValue(input);
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
      commentsError = errMessage(err, 'Failed to add process note');
    } finally {
      ctx.setBusy(false);
      if (!ctx.isDisposed()) ctx.paint();
    }
  }

  function beginEdit(commentId: string) {
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

  async function saveEdit(commentId: string) {
    const selectedMasterId = ctx.getSelectedMasterId();
    if (!selectedMasterId || ctx.isBusy() || !commentId) return;
    const editInput = ctx.getContainer().querySelector('[data-comment-edit-input]');
    const body = elementValue(editInput);
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
      commentsError = errMessage(err, 'Failed to update process note');
    } finally {
      ctx.setBusy(false);
      if (!ctx.isDisposed()) ctx.paint();
    }
  }

  function openDeleteConfirm(commentId: string) {
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

  async function confirmDelete(commentId: string) {
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
      commentsError = errMessage(err, 'Failed to delete process note');
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
    handleClick(event: Event, action: string, actionEl: HTMLElement | null) {
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
    handleKeydown(event: KeyboardEvent) {
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
