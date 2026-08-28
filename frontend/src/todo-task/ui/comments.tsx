import { renderToHtml } from '../../island.ts';
import { formatRelativeTime } from '../state/format.ts';
import type { TodoComment } from '../state/types.ts';

const COMMENTS_EMPTY_MSG = 'No process notes';

type CommentsUi = {
  comments?: TodoComment[];
  commentsError?: string;
  commentEditId?: string;
  commentDeleteConfirm?: string;
  disabled?: boolean;
};

export function renderCommentsSection(ui: CommentsUi) {
  return renderToHtml(<CommentsSection ui={ui} />);
}

export function CommentsSection({ ui }: { ui: CommentsUi }) {
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
