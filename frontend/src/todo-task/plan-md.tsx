// @ts-nocheck
import { renderToHtml } from '../island.ts';
import { renderCommentMarkdown } from '../shared/comment-markdown.ts';
import {
  COPY_MASTER_LABEL,
  COPY_MASTER_TITLE,
  formatMasterCopyText,
} from './format.ts';
import { readPlanMd, updatePlanMd } from './host.ts';

function CopyMasterIdButton({ master, disabled }) {
  const copyText = formatMasterCopyText(master?.title, master?.master_task_id);
  return (
    <button
      type="button"
      className="md-header-btn"
      data-action="copy-master-id"
      data-copy-text={copyText}
      title={COPY_MASTER_TITLE}
      disabled={disabled}
    >
      {COPY_MASTER_LABEL}
    </button>
  );
}

export function renderPlanMdSection(master, ui) {
  return renderToHtml(<PlanMdSection master={master} ui={ui} />);
}

export function PlanMdSection({ master, ui }) {
  const copyBtn = <CopyMasterIdButton master={master} disabled={ui.disabled} />;
  if (ui.planMdLoading) {
    return (
      <section className="todo-task-plan-md-section" aria-label="Todo description">
        <div className="todo-task-plan-md-header">
          <h3 className="todo-task-plan-md-title">Todo description</h3>
          <div className="todo-task-plan-md-header-actions">{copyBtn}</div>
        </div>
        <p className="todo-task-plan-md-loading">Loading description…</p>
      </section>
    );
  }
  if (ui.planMdEditMode) {
    return (
      <section className="todo-task-plan-md-section" aria-label="Todo description">
        <div className="todo-task-plan-md-header">
          <h3 className="todo-task-plan-md-title">Todo description</h3>
          <div className="todo-task-plan-md-header-actions">{copyBtn}</div>
        </div>
        {ui.planMdError ? (
          <p className="todo-task-plan-md-error" role="alert">
            {ui.planMdError}
          </p>
        ) : null}
        <textarea
          className="todo-task-plan-md-editor"
          disabled={ui.disabled}
          defaultValue={ui.planMdDraft ?? ''}
        />
        <div className="todo-task-plan-md-toolbar">
          <button
            type="button"
            className="md-header-btn primary"
            data-action="save-plan-md"
            disabled={ui.disabled}
          >
            Save
          </button>
          <button
            type="button"
            className="md-header-btn"
            data-action="cancel-plan-md"
            disabled={ui.disabled}
          >
            Cancel
          </button>
        </div>
      </section>
    );
  }
  const planMd = master.todo_md ?? '';
  const previewHtml = planMd ? renderCommentMarkdown(planMd) : '';
  return (
    <section className="todo-task-plan-md-section" aria-label="Todo description">
      <div className="todo-task-plan-md-header">
        <h3 className="todo-task-plan-md-title">Todo description</h3>
        <div className="todo-task-plan-md-header-actions">
          {copyBtn}
          <button
            type="button"
            className="md-header-btn"
            data-action="edit-plan-md"
            disabled={ui.disabled}
          >
            Edit
          </button>
        </div>
      </div>
      {previewHtml ? (
        <div
          className="todo-task-plan-md-preview"
          dangerouslySetInnerHTML={{ __html: previewHtml }}
        />
      ) : (
        <div className="todo-task-plan-md-preview">
          <p className="todo-task-plan-md-empty">No description</p>
        </div>
      )}
    </section>
  );
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
