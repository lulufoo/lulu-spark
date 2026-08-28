import { renderToHtml } from '../../island.ts';
import { renderCommentMarkdown } from '../../shared/comment-markdown.ts';
import { formatRelativeTime } from '../state/format.ts';

const ATTACHMENTS_EMPTY_MSG = 'No attachments';

type Attachment = {
  file_name: string;
  added_at?: string;
};

type AttachmentsUi = {
  disabled?: boolean;
  attachments?: Attachment[];
  attachmentsError?: string;
  attachmentDeleteConfirm?: string;
};

type AttachmentEditorState = {
  fileName: string;
  content?: string;
  editMode?: boolean;
  error?: string;
  loading?: boolean;
};

export function renderAttachmentsSection(ui: AttachmentsUi) {
  return renderToHtml(<AttachmentsSection ui={ui} />);
}

export function renderAttachmentEditor(editor: AttachmentEditorState, disabled?: boolean) {
  // Preview-default modal: attachment-preview unless editMode is set.
  return renderToHtml(<AttachmentEditor editor={editor} disabled={disabled} />);
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

export function AttachmentEditor({
  editor,
  disabled,
}: {
  editor: AttachmentEditorState;
  disabled?: boolean;
}) {
  const modeLabel = editor.loading ? 'Loading' : editor.editMode ? 'Edit' : 'Preview';
  const panelMode = editor.loading ? 'loading' : editor.editMode ? 'edit' : 'preview';
  const previewHtml = editor.content
    ? renderCommentMarkdown(editor.content)
    : '';
  return (
    <div
      className="todo-task-attachment-editor"
      role="dialog"
      aria-modal="true"
      aria-label={editor.fileName}
    >
      <div
        className="todo-task-attachment-editor-backdrop"
        data-action="close-attachment-editor"
      />
      <div className={`todo-task-attachment-editor-panel todo-task-attachment-editor-panel--${panelMode}`}>
        <header className="todo-task-attachment-editor-header">
          <div className="todo-task-attachment-editor-heading">
            <span className="todo-task-attachment-icon" aria-hidden="true">
              MD
            </span>
            <div className="todo-task-attachment-editor-title-wrap">
              <h3 className="todo-task-attachment-editor-title">{editor.fileName}</h3>
              <span className="todo-task-attachment-editor-mode">{modeLabel}</span>
            </div>
          </div>
          <div className="todo-task-attachment-editor-actions">
            {!editor.loading && !editor.editMode ? (
              <button
                type="button"
                className="md-header-btn primary"
                data-action="edit-attachment"
                disabled={disabled}
              >
                Edit
              </button>
            ) : null}
            <button
              type="button"
              className="md-header-btn"
              data-action="close-attachment-editor"
              disabled={disabled}
            >
              Close
            </button>
          </div>
        </header>
        <div className={`todo-task-attachment-editor-body todo-task-attachment-editor-body--${panelMode}`}>
          {editor.loading ? (
            <div className="todo-task-attachment-loading" role="status">
              <span className="todo-task-attachment-loading-dot" aria-hidden="true" />
              <span>Loading attachment…</span>
            </div>
          ) : editor.editMode ? (
            <>
              {editor.error ? (
                <p className="todo-task-attachment-error" role="alert">
                  {editor.error}
                </p>
              ) : null}
              <label className="todo-task-attachment-edit-label" htmlFor="todo-task-attachment-edit-area">
                Markdown source
              </label>
              <textarea
                id="todo-task-attachment-edit-area"
                className="todo-task-attachment-edit-area"
                spellCheck={false}
                disabled={disabled}
                defaultValue={editor.content ?? ''}
              />
            </>
          ) : (
            <>
              {editor.error ? (
                <p className="todo-task-attachment-error" role="alert">
                  {editor.error}
                </p>
              ) : null}
              <article className="todo-task-attachment-doc">
                {previewHtml ? (
                  <div
                    className="todo-task-attachment-preview"
                    dangerouslySetInnerHTML={{ __html: previewHtml }}
                  />
                ) : (
                  <div className="todo-task-attachment-preview">
                    <p className="todo-task-attachment-empty">No content</p>
                  </div>
                )}
              </article>
            </>
          )}
        </div>
        {editor.editMode && !editor.loading ? (
          <footer className="todo-task-attachment-editor-footer">
            <span className="todo-task-attachment-editor-footer-hint">
              Save will overwrite this attachment
            </span>
            <div className="todo-task-attachment-toolbar">
              <button
                type="button"
                className="md-header-btn"
                data-action="cancel-attachment-edit"
                disabled={disabled}
              >
                Cancel
              </button>
              <button
                type="button"
                className="md-header-btn primary"
                data-action="save-attachment"
                disabled={disabled}
              >
                Save
              </button>
            </div>
          </footer>
        ) : null}
      </div>
    </div>
  );
}
