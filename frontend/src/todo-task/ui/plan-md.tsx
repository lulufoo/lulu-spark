// @ts-nocheck
import { renderToHtml } from '../../island.ts';
import { renderCommentMarkdown } from '../../shared/comment-markdown.ts';
import {
  COPY_MASTER_LABEL,
  COPY_MASTER_TITLE,
  formatMasterCopyText,
} from '../state/format.ts';

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
