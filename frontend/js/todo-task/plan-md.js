import { escHtml } from '../utils.js';
import { renderCommentMarkdown } from '../comment-markdown.js';
import {
  COPY_MASTER_LABEL,
  COPY_MASTER_TITLE,
  escCopyDataAttr,
  formatMasterCopyText,
} from './format.js';
import { readPlanMd, updatePlanMd } from './host.js';

function renderCopyMasterIdButton(master, disabled) {
  const disabledAttr = disabled ? ' disabled' : '';
  const copyText = formatMasterCopyText(master?.title, master?.master_task_id);
  return `<button type="button" class="md-header-btn" data-action="copy-master-id" data-copy-text="${escCopyDataAttr(copyText)}" title="${COPY_MASTER_TITLE}"${disabledAttr}>${COPY_MASTER_LABEL}</button>`;
}

export function renderPlanMdSection(master, ui) {
  const disabledAttr = ui.disabled ? ' disabled' : '';
  const copyBtn = renderCopyMasterIdButton(master, ui.disabled);
  if (ui.planMdLoading) {
    return `
      <section class="todo-task-plan-md-section" aria-label="Todo description">
        <div class="todo-task-plan-md-header">
          <h3 class="todo-task-plan-md-title">Todo description</h3>
          <div class="todo-task-plan-md-header-actions">${copyBtn}</div>
        </div>
        <p class="todo-task-plan-md-loading">Loading description…</p>
      </section>
    `;
  }
  if (ui.planMdEditMode) {
    return `
      <section class="todo-task-plan-md-section" aria-label="Todo description">
        <div class="todo-task-plan-md-header">
          <h3 class="todo-task-plan-md-title">Todo description</h3>
          <div class="todo-task-plan-md-header-actions">${copyBtn}</div>
        </div>
        ${ui.planMdError ? `<p class="todo-task-plan-md-error" role="alert">${escHtml(ui.planMdError)}</p>` : ''}
        <textarea class="todo-task-plan-md-editor"${disabledAttr}>${escHtml(ui.planMdDraft ?? '')}</textarea>
        <div class="todo-task-plan-md-toolbar">
          <button type="button" class="md-header-btn primary" data-action="save-plan-md"${disabledAttr}>Save</button>
          <button type="button" class="md-header-btn" data-action="cancel-plan-md"${disabledAttr}>Cancel</button>
        </div>
      </section>
    `;
  }
  const planMd = master.todo_md ?? '';
  const previewHtml = planMd
    ? renderCommentMarkdown(planMd)
    : '<p class="todo-task-plan-md-empty">No description</p>';
  return `
    <section class="todo-task-plan-md-section" aria-label="Todo description">
      <div class="todo-task-plan-md-header">
        <h3 class="todo-task-plan-md-title">Todo description</h3>
        <div class="todo-task-plan-md-header-actions">
          ${copyBtn}
          <button type="button" class="md-header-btn" data-action="edit-plan-md"${disabledAttr}>Edit</button>
        </div>
      </div>
      <div class="todo-task-plan-md-preview">${previewHtml}</div>
    </section>
  `;
}

export function createPlanMdOwner(ctx) {
  let planMdEditMode = false;
  let planMdDraft = '';
  let planMdError = '';
  let planMdLoading = false;

  function reset() {
    planMdEditMode = false;
    planMdDraft = '';
    planMdError = '';
    planMdLoading = false;
  }

  return {
    uiSlice() {
      return { planMdEditMode, planMdDraft, planMdError, planMdLoading };
    },
    reset,
    async enterEdit() {
      const selectedMasterId = ctx.getSelectedMasterId();
      if (!selectedMasterId || ctx.isBusy()) return;
      planMdLoading = true;
      planMdError = '';
      ctx.paint();
      try {
        planMdDraft = await readPlanMd({ masterTaskId: selectedMasterId });
        planMdEditMode = true;
      } catch (err) {
        planMdError = err?.message || 'Failed to load description';
        planMdEditMode = false;
      } finally {
        planMdLoading = false;
        ctx.paint();
      }
    },
    async save() {
      const selectedMasterId = ctx.getSelectedMasterId();
      if (!selectedMasterId || ctx.isBusy()) return;
      const editor = ctx.getContainer().querySelector('.todo-task-plan-md-editor');
      planMdDraft = editor instanceof HTMLTextAreaElement ? editor.value : planMdDraft;
      planMdError = '';
      ctx.setBusy(true);
      ctx.paint();
      try {
        await updatePlanMd({ masterTaskId: selectedMasterId, planMd: planMdDraft });
        reset();
        ctx.setBusy(false);
        await ctx.reloadList({ afterWrite: true });
      } catch (err) {
        ctx.setBusy(false);
        planMdEditMode = true;
        planMdError = err?.message || 'Save failed';
        ctx.paint();
      }
    },
    cancel() {
      if (ctx.isBusy()) return;
      reset();
      ctx.paint();
    },
    handleClick(action) {
      if (action === 'edit-plan-md') {
        void this.enterEdit();
        return true;
      }
      if (action === 'save-plan-md') {
        void this.save();
        return true;
      }
      if (action === 'cancel-plan-md') {
        this.cancel();
        return true;
      }
      return false;
    },
  };
}
