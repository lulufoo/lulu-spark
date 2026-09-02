import { renderToHtml } from '../../island.ts';
import { formatRelativeTime } from '../state/format.ts';

const ATTACHMENTS_EMPTY_MSG = 'No attachments';

type Attachment = {
  file_name: string;
  added_at?: string;
  path?: string;
};

type AttachmentsUi = {
  disabled?: boolean;
  attachments?: Attachment[];
  attachmentsError?: string;
  attachmentDeleteConfirm?: string;
};

export function renderAttachmentsSection(ui: AttachmentsUi) {
  return renderToHtml(<AttachmentsSection ui={ui} />);
}

export function AttachmentsSection({ ui }: { ui: AttachmentsUi }) {
  const items = ui.attachments ?? [];
  const confirmFile = ui.attachmentDeleteConfirm || '';
  return (
    <section className="todo-task-attachments-section" aria-label="Attachments">
      <div className="todo-task-attachments-header">
        <div className="todo-task-attachments-heading">
          <h3 className="todo-task-attachments-title">Attachments</h3>
          {items.length ? (
            <span className="todo-task-attachments-count" aria-label={`${items.length} attachments`}>
              {items.length}
            </span>
          ) : null}
        </div>
        <div className="todo-task-attachments-header-actions">
          <button
            type="button"
            className="md-header-btn"
            data-action="pick-attachment-md"
            disabled={ui.disabled}
          >
            Choose local .md
          </button>
          {items.length ? null : (
            <span className="todo-task-attachments-empty">{ATTACHMENTS_EMPTY_MSG}</span>
          )}
        </div>
      </div>
      {ui.attachmentsError ? (
        <p className="todo-task-attachments-error" role="alert">
          {ui.attachmentsError}
        </p>
      ) : null}
      {items.length ? (
        <ul className="todo-task-attachment-list" role="list">
          {items.map((entry) => {
            const added = formatRelativeTime(entry.added_at);
            const meta = added ? `Added ${added}` : 'Markdown attachment';
            return (
              <li
                key={entry.file_name}
                className="todo-task-attachment-item"
                data-action="open-attachment"
                data-file-name={entry.file_name}
                data-path={entry.path || ''}
                role="button"
                tabIndex={0}
                aria-label={`Open attachment ${entry.file_name}`}
              >
                <span className="todo-task-attachment-icon" aria-hidden="true">
                  MD
                </span>
                <span className="todo-task-attachment-main">
                  <span className="todo-task-attachment-name">{entry.file_name}</span>
                  <span className="todo-task-attachment-meta">{meta}</span>
                </span>
                <span className="todo-task-attachment-open-hint" aria-hidden="true">
                  Open
                </span>
                <button
                  type="button"
                  className="todo-task-attachment-delete"
                  data-action="delete-attachment"
                  data-file-name={entry.file_name}
                  aria-label={`Delete attachment ${entry.file_name}`}
                  title="Delete"
                  disabled={ui.disabled}
                >
                  Delete
                </button>
              </li>
            );
          })}
        </ul>
      ) : null}
      {confirmFile ? (
        <div
          className="todo-task-attachment-delete-confirm"
          data-attachment-delete-confirm=""
          role="dialog"
          aria-modal="true"
          aria-label="Delete attachment confirmation"
        >
          <div
            className="todo-task-attachment-delete-confirm-backdrop"
            data-action="cancel-delete-attachment"
          />
          <div className="todo-task-attachment-delete-confirm-panel">
            <h4 className="todo-task-attachment-delete-confirm-title">Delete attachment</h4>
            <p className="todo-task-attachment-delete-confirm-body">
              Delete “{confirmFile}”? The list entry and file will both be removed.
            </p>
            <div className="todo-task-attachment-delete-confirm-actions">
              <button
                type="button"
                className="md-header-btn"
                data-action="cancel-delete-attachment"
                disabled={ui.disabled}
              >
                Cancel
              </button>
              <button
                type="button"
                className="md-header-btn todo-task-btn-danger"
                data-action="confirm-delete-attachment"
                data-file-name={confirmFile}
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
