import { escHtml } from '../shared/utils.js';
import { renderCommentMarkdown } from '../shared/comment-markdown.js';
import { formatRelativeTime } from './format.js';

const ATTACHMENTS_EMPTY_MSG = 'No attachments';

export function renderAttachmentsSection(ui) {
  const disabledAttr = ui.disabled ? ' disabled' : '';
  const items = ui.attachments ?? [];
  const confirmFile = ui.attachmentDeleteConfirm || '';
  const countHtml = items.length
    ? `<span class="todo-task-attachments-count" aria-label="${items.length} attachments">${items.length}</span>`
    : '';
  const emptyHtml = items.length
    ? ''
    : `<span class="todo-task-attachments-empty">${escHtml(ATTACHMENTS_EMPTY_MSG)}</span>`;
  const listHtml = items.length
    ? `<ul class="todo-task-attachment-list" role="list">${items
        .map((entry) => {
          const added = formatRelativeTime(entry.added_at);
          const meta = added ? `Added ${added}` : 'Markdown attachment';
          return `
        <li
          class="todo-task-attachment-item"
          data-action="open-attachment"
          data-file-name="${escHtml(entry.file_name)}"
          role="button"
          tabindex="0"
          aria-label="Open attachment ${escHtml(entry.file_name)}"
        >
          <span class="todo-task-attachment-icon" aria-hidden="true">MD</span>
          <span class="todo-task-attachment-main">
            <span class="todo-task-attachment-name">${escHtml(entry.file_name)}</span>
            <span class="todo-task-attachment-meta">${escHtml(meta)}</span>
          </span>
          <span class="todo-task-attachment-open-hint" aria-hidden="true">Open</span>
          <button
            type="button"
            class="todo-task-attachment-delete"
            data-action="delete-attachment"
            data-file-name="${escHtml(entry.file_name)}"
            aria-label="Delete attachment ${escHtml(entry.file_name)}"
            title="Delete"
            ${disabledAttr}
          >Delete</button>
        </li>`;
        })
        .join('')}</ul>`
    : '';
  const errHtml = ui.attachmentsError
    ? `<p class="todo-task-attachments-error" role="alert">${escHtml(ui.attachmentsError)}</p>`
    : '';
  const confirmHtml = confirmFile
    ? `
      <div
        class="todo-task-attachment-delete-confirm"
        data-attachment-delete-confirm
        role="dialog"
        aria-modal="true"
        aria-label="Delete attachment confirmation"
      >
        <div class="todo-task-attachment-delete-confirm-backdrop" data-action="cancel-delete-attachment"></div>
        <div class="todo-task-attachment-delete-confirm-panel">
          <h4 class="todo-task-attachment-delete-confirm-title">Delete attachment</h4>
          <p class="todo-task-attachment-delete-confirm-body">Delete “${escHtml(confirmFile)}”? The list entry and file will both be removed.</p>
          <div class="todo-task-attachment-delete-confirm-actions">
            <button
              type="button"
              class="md-header-btn"
              data-action="cancel-delete-attachment"
              ${disabledAttr}
            >Cancel</button>
            <button
              type="button"
              class="md-header-btn todo-task-btn-danger"
              data-action="confirm-delete-attachment"
              data-file-name="${escHtml(confirmFile)}"
              ${disabledAttr}
            >Confirm delete</button>
          </div>
        </div>
      </div>`
    : '';
  return `
    <section class="todo-task-attachments-section" aria-label="Attachments">
      <div class="todo-task-attachments-header">
        <div class="todo-task-attachments-heading">
          <h3 class="todo-task-attachments-title">Attachments</h3>
          ${countHtml}
        </div>
        <div class="todo-task-attachments-header-actions">
          <button type="button" class="md-header-btn" data-action="pick-attachment-md"${disabledAttr}>Choose local .md</button>
          ${emptyHtml}
        </div>
      </div>
      ${errHtml}
      ${listHtml}
      ${confirmHtml}
    </section>
  `;
}

export function renderAttachmentEditor(editor, disabled) {
  const disabledAttr = disabled ? ' disabled' : '';
  const modeLabel = editor.loading ? 'Loading' : editor.editMode ? 'Edit' : 'Preview';
  const panelMode = editor.loading
    ? 'loading'
    : editor.editMode
      ? 'edit'
      : 'preview';
  const errHtml = editor.error
    ? `<p class="todo-task-attachment-error" role="alert">${escHtml(editor.error)}</p>`
    : '';
  let bodyHtml;
  let footerHtml = '';
  if (editor.loading) {
    bodyHtml = `
      <div class="todo-task-attachment-loading" role="status">
        <span class="todo-task-attachment-loading-dot" aria-hidden="true"></span>
        <span>Loading attachment…</span>
      </div>`;
  } else if (editor.editMode) {
    bodyHtml = `
      ${errHtml}
      <label class="todo-task-attachment-edit-label" for="todo-task-attachment-edit-area">Markdown source</label>
      <textarea
        id="todo-task-attachment-edit-area"
        class="todo-task-attachment-edit-area"
        spellcheck="false"
        ${disabledAttr}
      >${escHtml(editor.content ?? '')}</textarea>
    `;
    footerHtml = `
      <footer class="todo-task-attachment-editor-footer">
        <span class="todo-task-attachment-editor-footer-hint">Save will overwrite this attachment</span>
        <div class="todo-task-attachment-toolbar">
          <button type="button" class="md-header-btn" data-action="cancel-attachment-edit"${disabledAttr}>Cancel</button>
          <button type="button" class="md-header-btn primary" data-action="save-attachment"${disabledAttr}>Save</button>
        </div>
      </footer>`;
  } else {
    const previewHtml = editor.content
      ? renderCommentMarkdown(editor.content)
      : '<p class="todo-task-attachment-empty">No content</p>';
    bodyHtml = `
      ${errHtml}
      <article class="todo-task-attachment-doc">
        <div class="todo-task-attachment-preview">${previewHtml}</div>
      </article>
    `;
  }
  const editBtn =
    !editor.loading && !editor.editMode
      ? `<button type="button" class="md-header-btn primary" data-action="edit-attachment"${disabledAttr}>Edit</button>`
      : '';
  return `
    <div class="todo-task-attachment-editor" role="dialog" aria-modal="true" aria-label="${escHtml(editor.fileName)}">
      <div class="todo-task-attachment-editor-backdrop" data-action="close-attachment-editor"></div>
      <div class="todo-task-attachment-editor-panel todo-task-attachment-editor-panel--${panelMode}">
        <header class="todo-task-attachment-editor-header">
          <div class="todo-task-attachment-editor-heading">
            <span class="todo-task-attachment-icon" aria-hidden="true">MD</span>
            <div class="todo-task-attachment-editor-title-wrap">
              <h3 class="todo-task-attachment-editor-title">${escHtml(editor.fileName)}</h3>
              <span class="todo-task-attachment-editor-mode">${escHtml(modeLabel)}</span>
            </div>
          </div>
          <div class="todo-task-attachment-editor-actions">
            ${editBtn}
            <button type="button" class="md-header-btn" data-action="close-attachment-editor"${disabledAttr}>Close</button>
          </div>
        </header>
        <div class="todo-task-attachment-editor-body todo-task-attachment-editor-body--${panelMode}">${bodyHtml}</div>
        ${footerHtml}
      </div>
    </div>
  `;
}

