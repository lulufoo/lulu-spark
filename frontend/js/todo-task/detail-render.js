import { escHtml } from '../shared/utils.js';
import { DEFAULT_PLAN_CATEGORY_ID } from './host.js';
import {
  categoryDisplayName,
  copySubIdPair,
  formatRelativeTime,
  formatTodoTaskStatus,
  masterCategoryId,
  masterStatusClass,
} from './format.js';
import { renderPlanMdSection } from './plan-md.js';
import { renderAttachmentsSection } from './attachments.js';
import { renderCommentsSection } from './comments.js';

const MIGRATION_WARNING_MSG = 'Todo migration incomplete; some data may be missing';
const REFRESH_WARNING_MSG = 'Saved, but list refresh failed — retry';

function renderLinkedArchives(linkedArchiveIds) {
  if (!linkedArchiveIds?.length) return '';
  return `<div class="todo-task-sub-archives">Linked archives:${escHtml(linkedArchiveIds.join(', '))}</div>`;
}

function renderSubStatusSelect(sub, disabled) {
  const disabledAttr = disabled || sub.status !== 'incomplete' ? ' disabled' : '';
  const options = ['incomplete', 'complete', 'abandoned']
    .map((status) => {
      const selected = sub.status === status ? ' selected' : '';
      return `<option value="${escHtml(status)}"${selected}>${escHtml(formatTodoTaskStatus(status))}</option>`;
    })
    .join('');
  return `
    <select
      class="todo-task-status-select todo-task-sub-status-select todo-task-sub-status-select--${escHtml(sub.status)}"
      data-action="change-sub-status"
      data-sub-id="${escHtml(sub.sub_task_id)}"
      aria-label="Sub-task status"${disabledAttr}
    >${options}</select>
  `;
}

function renderSubTitleError(error) {
  if (!error) return '';
  return `<p class="todo-task-sub-title-error" role="alert">${escHtml(error)}</p>`;
}

function renderSubActionError(error) {
  if (!error) return '';
  return `<p class="todo-task-sub-action-error" role="alert">${escHtml(error)}</p>`;
}

export function renderSubRow(master, sub, selectedSubId, ui) {
  const effectiveStatus = ui.subStatus?.[sub.sub_task_id] ?? sub.status;
  const subForRender = { ...sub, status: effectiveStatus };
  const selected = sub.sub_task_id === selectedSubId ? ' todo-task-sub--selected' : '';
  const title = ui.subTitleDrafts?.[sub.sub_task_id] ?? (sub.title || sub.sub_task_id);
  const disabledAttr = ui.disabled ? ' disabled' : '';
  const copyText = copySubIdPair(master.master_task_id, sub.sub_task_id);
  const subError = ui.subActionErrors?.[sub.sub_task_id] ?? '';
  const expanded = Boolean(ui.expandedSubContent?.[sub.sub_task_id]);
  const contentValue =
    ui.subContentDrafts?.[sub.sub_task_id] ?? (typeof sub.content === 'string' ? sub.content : '');
  const contentEditor = expanded
    ? `<textarea
          class="todo-task-sub-content-editor"
          data-action="edit-sub-content"
          data-sub-id="${escHtml(sub.sub_task_id)}"
          aria-label="Sub-task content"
          ${ui.disabled ? 'disabled' : ''}
        >${escHtml(contentValue)}</textarea>`
    : '';
  return `
    <article data-sub-id="${escHtml(sub.sub_task_id)}" class="todo-task-sub${selected}">
      <header class="todo-task-sub-header">
        <input
          type="text"
          class="todo-task-sub-title-input"
          data-action="edit-sub-title"
          data-sub-id="${escHtml(sub.sub_task_id)}"
          value="${escHtml(title)}"
          aria-label="Sub-task title"
          ${ui.disabled ? 'disabled' : ''}
        />
        <div class="todo-task-sub-header-actions">
          <button
            type="button"
            class="md-header-btn todo-task-sub-content-toggle"
            data-action="toggle-sub-content"
            data-sub-id="${escHtml(sub.sub_task_id)}"
            aria-expanded="${expanded ? 'true' : 'false'}"
            aria-label="Edit content"
            ${disabledAttr}
          >Content</button>
          ${renderSubStatusSelect(subForRender, ui.disabled)}
          <div class="todo-task-sub-menu">
            <button type="button" class="todo-task-sub-menu-btn" data-action="toggle-sub-menu" aria-label="More actions"${disabledAttr}>⋯</button>
            <div class="todo-task-sub-menu-panel" hidden>
              <button type="button" data-action="copy-sub-id" data-copy-text="${escHtml(copyText)}">Copy ID</button>
              <button type="button" data-action="delete-sub" data-sub-id="${escHtml(sub.sub_task_id)}" data-sub-title="${escHtml(title)}">Delete</button>
            </div>
          </div>
        </div>
      </header>
      ${renderSubTitleError(ui.subTitleErrors?.[sub.sub_task_id] ?? '')}
      ${renderSubActionError(subError)}
      ${contentEditor}
      ${renderLinkedArchives(sub.linked_archive_ids)}
    </article>
  `;
}

function renderMasterStatusSelect(status, disabled) {
  const wire = masterStatusClass(status);
  const disabledAttr = disabled ? ' disabled' : '';
  const options = ['incomplete', 'complete', 'abandoned']
    .map((value) => {
      const selected = wire === value ? ' selected' : '';
      return `<option value="${escHtml(value)}"${selected}>${escHtml(formatTodoTaskStatus(value))}</option>`;
    })
    .join('');
  return `
    <select
      class="todo-task-status-select todo-task-master-status-select todo-task-master-status-select--${escHtml(wire)}"
      data-action="change-master-status"
      aria-label="Todo status"${disabledAttr}
    >${options}</select>
  `;
}

function renderMasterCategorySelect(master, categories, disabled) {
  const current = masterCategoryId(master);
  const disabledAttr = disabled ? ' disabled' : '';
  const list = categories?.length
    ? categories
    : [{ id: DEFAULT_PLAN_CATEGORY_ID, name: 'Uncategorized', is_default: true }];
  const options = list
    .map((category) => {
      const selected = category.id === current ? ' selected' : '';
      return `<option value="${escHtml(category.id)}"${selected}>${escHtml(categoryDisplayName(category))}</option>`;
    })
    .join('');
  return `
    <select
      class="todo-task-category-select"
      data-action="change-master-category"
      aria-label="Todo category"${disabledAttr}
    >${options}</select>
  `;
}

export function renderDetailTitle(master, ui = {}) {
  const title = ui.masterTitleDraft ?? master.title ?? '';
  const status = masterStatusClass(master.status);
  const disabledAttr = ui.disabled ? ' disabled' : '';
  const error = ui.masterTitleError
    ? `<p class="todo-task-detail-title-error">${escHtml(ui.masterTitleError)}</p>`
    : '';
  return `
    <div class="todo-task-detail-title-row">
      <input
        type="text"
        class="todo-task-detail-title todo-task-detail-title--${escHtml(status)}"
        data-action="edit-master-title"
        value="${escHtml(title)}"
        aria-label="Todo title"
        ${disabledAttr}
      />
      ${renderMasterCategorySelect(master, ui.categories, ui.disabled)}
      ${renderMasterStatusSelect(status, ui.disabled)}
    </div>
    ${error}
  `;
}

export function renderSubDetail(master, selectedSubId) {
  const subs = master.sub_tasks ?? [];
  const ui = { disabled: false, subStatus: {}, subActionErrors: {} };
  const items = subs.map((sub) => renderSubRow(master, sub, selectedSubId, ui)).join('');
  return `
    <div class="todo-task-detail-body">
      ${renderDetailTitle(master, ui)}
      <div class="todo-task-sub-list">${items}</div>
    </div>
  `;
}

function renderDetailMeta(master) {
  const subCount = master.sub_tasks?.length ?? 0;
  const created = formatRelativeTime(master.created_at) || 'Unknown time';
  return `<p class="todo-task-detail-meta">${subCount} sub-tasks · created ${escHtml(created)}</p>`;
}

function renderDetailToolbar(masterTaskId, disabled) {
  const disabledAttr = disabled ? ' disabled' : '';
  return `
    <div class="todo-task-detail-toolbar">
      <button type="button" class="md-header-btn" data-action="add-sub" data-master-id="${escHtml(masterTaskId)}"${disabledAttr}>Add sub-task</button>
      <button type="button" class="md-header-btn todo-task-btn-danger" data-action="delete-master"${disabledAttr}>Delete todo</button>
    </div>
  `;
}

function renderSubEmpty(disabled) {
  const disabledAttr = disabled ? ' disabled' : '';
  return `
    <div class="todo-task-empty todo-task-empty--detail">
      <p class="todo-task-empty-title">No sub-tasks yet</p>
      <button type="button" class="md-header-btn primary" data-action="add-sub"${disabledAttr}>Add sub-task</button>
    </div>
  `;
}
function renderMigrationWarning() {
  return `
    <div class="todo-task-migration-warning todo-task-split-state" role="status">
      <p class="todo-task-split-state-detail">${escHtml(MIGRATION_WARNING_MSG)}</p>
    </div>
  `;
}

function renderRefreshWarning(refreshWarning, disabled) {
  if (!refreshWarning) return '';
  const disabledAttr = disabled ? ' disabled' : '';
  return `
    <div class="todo-task-refresh-warning todo-task-split-state" role="status">
      <p class="todo-task-split-state-detail">${escHtml(refreshWarning)}</p>
      <button type="button" class="md-header-btn todo-task-refresh-retry"${disabledAttr} data-action="retry-refresh">Retry refresh</button>
    </div>
  `;
}

export function renderSubDetailPane(master, selectedSubId, ui) {
  const subs = master.sub_tasks ?? [];
  const items = subs.length
    ? subs.map((sub) => renderSubRow(master, sub, selectedSubId, ui)).join('')
    : renderSubEmpty(ui.disabled);
  return `
    <div class="todo-task-detail-body">
      <div class="todo-task-detail-header">
        <div>
          ${renderDetailTitle(master, ui)}
          ${renderDetailMeta(master)}
        </div>
      </div>
      ${master.migration_error ? renderMigrationWarning() : ''}
      ${renderPlanMdSection(master, ui)}
      ${renderAttachmentsSection(ui)}
      ${renderRefreshWarning(ui.refreshWarning, ui.disabled)}
      ${renderDetailToolbar(master.master_task_id, ui.disabled)}
      <div class="todo-task-sub-list">${items}</div>
      ${renderCommentsSection(ui)}
    </div>
  `;
}

