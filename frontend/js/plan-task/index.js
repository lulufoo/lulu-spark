import { createApiClient, resolveReadDriver } from '../apiClient.js';
import { renderCommentMarkdown } from '../comment-markdown.js';
import { escHtml } from '../utils.js';
import { closePlanTaskDialog, openPlanTaskDialog } from './dialog.js';
export {
  buildTodosBinding,
  resetTodosBinding,
  assembleTodosBindingBody,
  TODOS_T_LIFT_TOOL_NAMES,
  TODOS_PLAN_ASSISTANT_PROMPT,
} from './todos-binding.js';
export {
  createTodosPageLifecycle,
  onTodosPageEnter,
  onMasterSelectionChange,
  onTodosPageLeave,
} from './todos-lifecycle.js';
import { createTodosPageLifecycle } from './todos-lifecycle.js';

const UNAVAILABLE_MSG = 'List temporarily unavailable. Please try again later.';
const REFRESH_WARNING_MSG = 'Saved, but list refresh failed — retry';
const MIGRATION_WARNING_MSG = 'Todo migration incomplete; some data may be missing';
const ATTACHMENTS_EMPTY_MSG = 'No attachments';
const ATTACHMENT_PICK_CANCEL_MSG = 'File selection cancelled';
const COMMENTS_EMPTY_MSG = 'No process notes';
const AI_ASSISTANT_TURN_COMPLETED = 'ai-assistant:turn-completed';
/**
 * t4 / N1: `open_ai_assistant(masterTaskId)` is not Todos executable success main path.
 * Page entry Present uses `present_ai_assistant`; Binding is via Binding Contract Set.
 */
export const TODOS_OPEN_AND_BIND_MAIN_PATH_DISABLED = true;
const COPY_MASTER_ID_LABEL = 'Copy task ID';
const COPY_FEEDBACK_LABEL = '✓ Copied';
const COPY_FEEDBACK_MS = 1200;

const STATUS_LABELS = {
  incomplete: 'In progress',
  complete: 'Completed',
  abandoned: 'Abandoned',
};

function getTauriInvoke() {
  if (typeof window === 'undefined') return null;
  const invoke =
    window.__TAURI__?.core?.invoke || window.__TAURI_INTERNALS__?.invoke;
  return typeof invoke === 'function' ? invoke : null;
}

function getTauriListen() {
  if (typeof window === 'undefined') return null;
  const listen = window.__TAURI__?.event?.listen;
  return typeof listen === 'function' ? listen : null;
}

function serviceError(data) {
  if (!data || typeof data !== 'object' || Array.isArray(data)) return null;
  if (!data.error) return null;
  const err = new Error(String(data.error));
  err.status = typeof data._status === 'number' ? data._status : 500;
  return err;
}

export async function loadPlanTasks() {
  const mode = resolveReadDriver();
  const client = createApiClient(resolveReadDriver(mode));
  const data = await client.getJson('/api/todo-tasks');
  const err = serviceError(data);
  if (err) throw err;
  return Array.isArray(data) ? data : [];
}

async function invokePlanWrite(command, args) {
  const invoke = getTauriInvoke();
  if (!invoke) {
    throw new Error('Tauri invoke unavailable');
  }
  const result = await invoke(command, args);
  const err = serviceError(result);
  if (err) throw err;
  return result;
}

export async function createPlanTask({ title, subTitles } = {}) {
  const args = { title };
  if (subTitles != null) {
    args.subTitles = subTitles;
  }
  return invokePlanWrite('create_todo_task', args);
}

export async function deletePlanTask({ masterTaskId } = {}) {
  return invokePlanWrite('delete_todo_task', { masterTaskId });
}

export async function addPlanSub({ masterTaskId, title } = {}) {
  return invokePlanWrite('add_todo_sub', { masterTaskId, title });
}

export async function deletePlanSub({ masterTaskId, subTaskId } = {}) {
  return invokePlanWrite('delete_todo_sub', { masterTaskId, subTaskId });
}

async function invokePlanPlain(command, args) {
  const invoke = getTauriInvoke();
  if (!invoke) {
    throw new Error('Tauri invoke unavailable');
  }
  const result = await invoke(command, args);
  const err = serviceError(result);
  if (err) throw err;
  return result;
}

export async function readPlanMd({ masterTaskId } = {}) {
  return invokePlanPlain('read_todo_md', { masterTaskId });
}

export async function updatePlanMd({ masterTaskId, planMd } = {}) {
  await invokePlanPlain('update_todo_md', { masterTaskId, planMd });
}

export async function completePlan({ masterTaskId, subTaskId } = {}) {
  return invokePlanWrite('complete_todo', { masterTaskId, subTaskId });
}

export async function abandonPlanSub({ masterTaskId, subTaskId } = {}) {
  return invokePlanWrite('abandon_todo_sub', { masterTaskId, subTaskId });
}

export async function updatePlanSub({ masterTaskId, subTaskId, title } = {}) {
  return invokePlanWrite('update_todo_sub', { masterTaskId, subTaskId, title });
}

export async function updatePlanMasterTitle({ masterTaskId, title } = {}) {
  return invokePlanWrite('update_todo_master_title', { masterTaskId, title });
}

export async function setPlanMasterStatus({ masterTaskId, status } = {}) {
  return invokePlanWrite('set_todo_master_status', { masterTaskId, status });
}

export async function listPlanAttachments({ masterTaskId } = {}) {
  const result = await invokePlanPlain('list_todo_attachments', { masterTaskId });
  if (Array.isArray(result)) return result;
  if (result && Array.isArray(result.attachments)) return result.attachments;
  return [];
}

export async function addPlanAttachment({ masterTaskId, fileName, content } = {}) {
  return invokePlanPlain('add_todo_attachment', { masterTaskId, fileName, content });
}

export async function readPlanAttachment({ masterTaskId, fileName } = {}) {
  return invokePlanPlain('read_todo_attachment', { masterTaskId, fileName });
}

export async function savePlanAttachment({ masterTaskId, fileName, content } = {}) {
  return invokePlanPlain('save_todo_attachment', { masterTaskId, fileName, content });
}

export async function deletePlanAttachment({ masterTaskId, fileName } = {}) {
  return invokePlanPlain('delete_todo_attachment', { masterTaskId, fileName });
}

export async function listPlanComments({ masterTaskId } = {}) {
  const result = await invokePlanPlain('list_todo_comments', { masterTaskId });
  if (Array.isArray(result)) return result;
  if (result && Array.isArray(result.comments)) return result.comments;
  return [];
}

export async function addPlanComment({ masterTaskId, body } = {}) {
  return invokePlanPlain('add_todo_comment', { masterTaskId, body });
}

export async function updatePlanComment({ masterTaskId, commentId, body } = {}) {
  return invokePlanPlain('update_todo_comment', { masterTaskId, commentId, body });
}

export async function deletePlanComment({ masterTaskId, commentId } = {}) {
  return invokePlanPlain('delete_todo_comment', { masterTaskId, commentId });
}

function basenameFromPath(path) {
  const normalized = String(path).replace(/\\/g, '/');
  const parts = normalized.split('/').filter(Boolean);
  return parts[parts.length - 1] || 'attachment.md';
}

/**
 * Host dialog: pick a local `.md`, then read content via convertFileSrc asset URL.
 * @returns {Promise<{ fileName: string, content: string } | null>} null when cancelled
 */
export async function pickLocalMarkdownFile() {
  if (typeof window === 'undefined') {
    throw new Error('File picker unavailable');
  }
  const dialog = window.__TAURI__?.dialog;
  if (typeof dialog?.open !== 'function') {
    throw new Error('File picker unavailable');
  }
  const selected = await dialog.open({
    multiple: false,
    filters: [{ name: 'Markdown', extensions: ['md'] }],
  });
  if (selected == null || selected === '') {
    return null;
  }
  const path = Array.isArray(selected) ? selected[0] : selected;
  if (!path) return null;

  const convertFileSrc = window.__TAURI__?.core?.convertFileSrc;
  const readAsset = typeof window['fetch'] === 'function' ? window['fetch'].bind(window) : null;
  if (typeof convertFileSrc !== 'function' || !readAsset) {
    throw new Error('Could not read selected file');
  }
  const res = await readAsset(convertFileSrc(path));
  if (!res.ok) {
    throw new Error('Could not read selected file');
  }
  return { fileName: basenameFromPath(path), content: await res.text() };
}

export function copySubIdPair(masterId, subId) {
  return `${masterId} → ${subId}`;
}

export function formatPlanTaskStatus(status) {
  return STATUS_LABELS[status] ?? status;
}

function buildDeepLink(masterId, subId) {
  const params = new URLSearchParams({ master: masterId, sub: subId });
  return `#/plan-tasks?${params.toString()}`;
}

function buildMasterDeepLink(masterId) {
  const params = new URLSearchParams({ master: masterId });
  return `#/plan-tasks?${params.toString()}`;
}

function sortMasters(masters) {
  return [...masters].sort((a, b) => {
    const aTime = Date.parse(a.created_at ?? '') || 0;
    const bTime = Date.parse(b.created_at ?? '') || 0;
    return bTime - aTime;
  });
}

function pickDefaultSub(master) {
  const subs = master.sub_tasks ?? [];
  const incomplete = subs.find((sub) => sub.status !== 'complete');
  return incomplete ?? subs[0] ?? null;
}

function formatRelativeTime(iso) {
  const time = Date.parse(iso ?? '');
  if (!time) return '';
  const diffMs = Date.now() - time;
  const minutes = Math.floor(diffMs / 60000);
  if (minutes < 1) return 'Just now';
  if (minutes < 60) return `${minutes} minutes ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} hours ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days} days ago`;
  return new Date(time).toLocaleDateString('en-US');
}

function controlsDisabled(busy) {
  return busy;
}

function renderMigrationWarning() {
  return `
    <div class="plan-task-migration-warning plan-task-split-state" role="status">
      <p class="plan-task-split-state-detail">${escHtml(MIGRATION_WARNING_MSG)}</p>
    </div>
  `;
}

function renderRefreshWarning(refreshWarning, disabled) {
  if (!refreshWarning) return '';
  const disabledAttr = disabled ? ' disabled' : '';
  return `
    <div class="plan-task-refresh-warning plan-task-split-state" role="status">
      <p class="plan-task-split-state-detail">${escHtml(refreshWarning)}</p>
      <button type="button" class="md-header-btn plan-task-refresh-retry"${disabledAttr} data-action="retry-refresh">Retry refresh</button>
    </div>
  `;
}

const ACTIVE_ONLY_LABEL = 'Active only';
const ACTIVE_ONLY_EMPTY_TITLE = 'No active todos';
const ACTIVE_ONLY_EMPTY_DETAIL =
  'Turn off Active only to see completed and abandoned todos.';

function isIncompleteMaster(master) {
  return masterStatusClass(master?.status) === 'incomplete';
}

function filterMastersForView(masters, activeOnly) {
  if (!activeOnly) return masters;
  return masters.filter(isIncompleteMaster);
}

function renderPageHeader(disabled, activeOnly = true) {
  const disabledAttr = disabled ? ' disabled' : '';
  const checked = activeOnly ? 'true' : 'false';
  return `
    <header class="plan-tasks-page-header">
      <h1 class="plan-tasks-page-title">Todos</h1>
      <div class="plan-tasks-page-header-actions">
        <label class="plan-task-active-only">
          <span class="plan-task-active-only-label">${ACTIVE_ONLY_LABEL}</span>
          <button
            type="button"
            class="plan-task-active-only-switch"
            role="switch"
            data-action="toggle-active-only"
            aria-checked="${checked}"
            aria-label="${ACTIVE_ONLY_LABEL}"${disabledAttr}
          ></button>
        </label>
        <button type="button" class="md-header-btn primary" data-action="create-master"${disabledAttr}>+ New todo</button>
      </div>
    </header>
  `;
}

function masterStatusClass(status) {
  const wire = status === 'complete' || status === 'abandoned' ? status : 'incomplete';
  return wire;
}

function renderMasterList(masters, selectedMasterId, disabled) {
  const disabledAttr = disabled ? ' disabled' : '';
  const items = sortMasters(masters)
    .map((master) => {
      const selected =
        master.master_task_id === selectedMasterId ? ' plan-task-master-item--selected' : '';
      const statusMod = ` plan-task-master-item--${masterStatusClass(master.status)}`;
      const subCount = master.sub_tasks?.length ?? 0;
      const meta = `${subCount} sub-tasks · ${formatRelativeTime(master.created_at) || 'Unknown time'}`;
      return `
        <li>
          <button type="button" class="plan-task-master-item${selected}${statusMod}" data-master-id="${escHtml(master.master_task_id)}"${disabledAttr}>
            <span class="plan-task-master-title">${escHtml(master.title)}</span>
            <span class="plan-task-master-meta">${escHtml(meta)}</span>
          </button>
        </li>
      `;
    })
    .join('');
  return `<ul class="plan-task-master-list" role="list">${items}</ul>`;
}

function renderMasterEmpty(disabled) {
  const disabledAttr = disabled ? ' disabled' : '';
  return `
    <div class="plan-task-empty plan-task-empty--sidebar">
      <p class="plan-task-empty-title">No todos yet</p>
      <p class="plan-task-empty-detail">Create your first todo to manage sub-tasks</p>
      <button type="button" class="md-header-btn primary" data-action="create-master"${disabledAttr}>New todo</button>
    </div>
  `;
}

function renderActiveOnlyEmpty() {
  return `
    <div class="plan-task-empty plan-task-empty--sidebar">
      <p class="plan-task-empty-title">${ACTIVE_ONLY_EMPTY_TITLE}</p>
      <p class="plan-task-empty-detail">${ACTIVE_ONLY_EMPTY_DETAIL}</p>
    </div>
  `;
}

function renderMasterPane(masters, selectedMasterId, disabled, activeOnly = true) {
  const visible = filterMastersForView(masters, activeOnly);
  if (!masters.length) {
    return renderMasterEmpty(disabled);
  }
  if (!visible.length) {
    return renderActiveOnlyEmpty();
  }
  return renderMasterList(visible, selectedMasterId, disabled);
}

function renderLinkedArchives(linkedArchiveIds) {
  if (!linkedArchiveIds?.length) return '';
  return `<div class="plan-task-sub-archives">Linked archives:${escHtml(linkedArchiveIds.join(', '))}</div>`;
}

function renderSubStatusSelect(sub, disabled) {
  const disabledAttr = disabled || sub.status !== 'incomplete' ? ' disabled' : '';
  const options = ['incomplete', 'complete', 'abandoned']
    .map((status) => {
      const selected = sub.status === status ? ' selected' : '';
      return `<option value="${escHtml(status)}"${selected}>${escHtml(formatPlanTaskStatus(status))}</option>`;
    })
    .join('');
  return `
    <select
      class="plan-task-status-select plan-task-sub-status-select plan-task-sub-status-select--${escHtml(sub.status)}"
      data-action="change-sub-status"
      data-sub-id="${escHtml(sub.sub_task_id)}"
      aria-label="Sub-task status"${disabledAttr}
    >${options}</select>
  `;
}

function renderSubTitleError(error) {
  if (!error) return '';
  return `<p class="plan-task-sub-title-error" role="alert">${escHtml(error)}</p>`;
}

function renderSubActionError(error) {
  if (!error) return '';
  return `<p class="plan-task-sub-action-error" role="alert">${escHtml(error)}</p>`;
}

function renderSubRow(master, sub, selectedSubId, ui) {
  const effectiveStatus = ui.subStatus?.[sub.sub_task_id] ?? sub.status;
  const subForRender = { ...sub, status: effectiveStatus };
  const selected = sub.sub_task_id === selectedSubId ? ' plan-task-sub--selected' : '';
  const title = ui.subTitleDrafts?.[sub.sub_task_id] ?? (sub.title || sub.sub_task_id);
  const disabledAttr = ui.disabled ? ' disabled' : '';
  const copyText = copySubIdPair(master.master_task_id, sub.sub_task_id);
  const subError = ui.subActionErrors?.[sub.sub_task_id] ?? '';
  return `
    <article data-sub-id="${escHtml(sub.sub_task_id)}" class="plan-task-sub${selected}">
      <header class="plan-task-sub-header">
        <input
          type="text"
          class="plan-task-sub-title-input"
          data-action="edit-sub-title"
          data-sub-id="${escHtml(sub.sub_task_id)}"
          value="${escHtml(title)}"
          aria-label="Sub-task title"
          ${ui.disabled ? 'disabled' : ''}
        />
        <div class="plan-task-sub-header-actions">
          ${renderSubStatusSelect(subForRender, ui.disabled)}
          <div class="plan-task-sub-menu">
            <button type="button" class="plan-task-sub-menu-btn" data-action="toggle-sub-menu" aria-label="More actions"${disabledAttr}>⋯</button>
            <div class="plan-task-sub-menu-panel" hidden>
              <button type="button" data-action="copy-sub-id" data-copy-text="${escHtml(copyText)}">Copy ID</button>
              <button type="button" data-action="delete-sub" data-sub-id="${escHtml(sub.sub_task_id)}" data-sub-title="${escHtml(title)}">Delete</button>
            </div>
          </div>
        </div>
      </header>
      ${renderSubTitleError(ui.subTitleErrors?.[sub.sub_task_id] ?? '')}
      ${renderSubActionError(subError)}
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
      return `<option value="${escHtml(value)}"${selected}>${escHtml(formatPlanTaskStatus(value))}</option>`;
    })
    .join('');
  return `
    <select
      class="plan-task-status-select plan-task-master-status-select plan-task-master-status-select--${escHtml(wire)}"
      data-action="change-master-status"
      aria-label="Todo status"${disabledAttr}
    >${options}</select>
  `;
}

function renderDetailTitle(master, ui = {}) {
  const title = ui.masterTitleDraft ?? master.title ?? '';
  const status = masterStatusClass(master.status);
  const disabledAttr = ui.disabled ? ' disabled' : '';
  const error = ui.masterTitleError
    ? `<p class="plan-task-detail-title-error">${escHtml(ui.masterTitleError)}</p>`
    : '';
  return `
    <div class="plan-task-detail-title-row">
      <input
        type="text"
        class="plan-task-detail-title plan-task-detail-title--${escHtml(status)}"
        data-action="edit-master-title"
        value="${escHtml(title)}"
        aria-label="Todo title"
        ${disabledAttr}
      />
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
    <div class="plan-task-detail-body">
      ${renderDetailTitle(master, ui)}
      <div class="plan-task-sub-list">${items}</div>
    </div>
  `;
}

function renderDetailMeta(master) {
  const subCount = master.sub_tasks?.length ?? 0;
  const created = formatRelativeTime(master.created_at) || 'Unknown time';
  return `<p class="plan-task-detail-meta">${subCount} sub-tasks · created ${escHtml(created)}</p>`;
}

function renderDetailToolbar(masterTaskId, disabled) {
  const disabledAttr = disabled ? ' disabled' : '';
  return `
    <div class="plan-task-detail-toolbar">
      <button type="button" class="md-header-btn" data-action="open-ai-assistant" data-master-id="${escHtml(masterTaskId)}"${disabledAttr}>Assistant</button>
      <button type="button" class="md-header-btn" data-action="add-sub" data-master-id="${escHtml(masterTaskId)}"${disabledAttr}>Add sub-task</button>
      <button type="button" class="md-header-btn plan-task-btn-danger" data-action="delete-master"${disabledAttr}>Delete todo</button>
    </div>
  `;
}

function renderSubEmpty(disabled) {
  const disabledAttr = disabled ? ' disabled' : '';
  return `
    <div class="plan-task-empty plan-task-empty--detail">
      <p class="plan-task-empty-title">No sub-tasks yet</p>
      <button type="button" class="md-header-btn primary" data-action="add-sub"${disabledAttr}>Add sub-task</button>
    </div>
  `;
}

function renderCopyMasterIdButton(masterTaskId, disabled) {
  const disabledAttr = disabled ? ' disabled' : '';
  return `<button type="button" class="md-header-btn" data-action="copy-master-id" data-copy-text="${escHtml(masterTaskId)}" title="${COPY_MASTER_ID_LABEL}"${disabledAttr}>${COPY_MASTER_ID_LABEL}</button>`;
}

function flashCopyFeedback(btn, restoreLabel) {
  if (!(btn instanceof HTMLElement)) return;
  const prev = btn._copyFeedbackTimer;
  if (prev) clearTimeout(prev);
  btn.textContent = COPY_FEEDBACK_LABEL;
  btn.classList.remove('plan-task-copy-flash');
  void btn.offsetWidth;
  btn.classList.add('plan-task-copy-flash');
  btn._copyFeedbackTimer = setTimeout(() => {
    btn.textContent = restoreLabel;
    btn.classList.remove('plan-task-copy-flash');
    btn._copyFeedbackTimer = null;
  }, COPY_FEEDBACK_MS);
}

function renderPlanMdSection(master, ui) {
  const disabledAttr = ui.disabled ? ' disabled' : '';
  const copyBtn = renderCopyMasterIdButton(master.master_task_id, ui.disabled);
  if (ui.planMdLoading) {
    return `
      <section class="plan-task-plan-md-section" aria-label="Todo description">
        <div class="plan-task-plan-md-header">
          <h3 class="plan-task-plan-md-title">Todo description</h3>
          <div class="plan-task-plan-md-header-actions">${copyBtn}</div>
        </div>
        <p class="plan-task-plan-md-loading">Loading description…</p>
      </section>
    `;
  }
  if (ui.planMdEditMode) {
    return `
      <section class="plan-task-plan-md-section" aria-label="Todo description">
        <div class="plan-task-plan-md-header">
          <h3 class="plan-task-plan-md-title">Todo description</h3>
          <div class="plan-task-plan-md-header-actions">${copyBtn}</div>
        </div>
        ${ui.planMdError ? `<p class="plan-task-plan-md-error" role="alert">${escHtml(ui.planMdError)}</p>` : ''}
        <textarea class="plan-task-plan-md-editor"${disabledAttr}>${escHtml(ui.planMdDraft ?? '')}</textarea>
        <div class="plan-task-plan-md-toolbar">
          <button type="button" class="md-header-btn primary" data-action="save-plan-md"${disabledAttr}>Save</button>
          <button type="button" class="md-header-btn" data-action="cancel-plan-md"${disabledAttr}>Cancel</button>
        </div>
      </section>
    `;
  }
  const planMd = master.todo_md ?? '';
  const previewHtml = planMd
    ? renderCommentMarkdown(planMd)
    : '<p class="plan-task-plan-md-empty">No description</p>';
  return `
    <section class="plan-task-plan-md-section" aria-label="Todo description">
      <div class="plan-task-plan-md-header">
        <h3 class="plan-task-plan-md-title">Todo description</h3>
        <div class="plan-task-plan-md-header-actions">
          ${copyBtn}
          <button type="button" class="md-header-btn" data-action="edit-plan-md"${disabledAttr}>Edit</button>
        </div>
      </div>
      <div class="plan-task-plan-md-preview">${previewHtml}</div>
    </section>
  `;
}

function renderAttachmentsSection(ui) {
  const disabledAttr = ui.disabled ? ' disabled' : '';
  const items = ui.attachments ?? [];
  const confirmFile = ui.attachmentDeleteConfirm || '';
  const countHtml = items.length
    ? `<span class="plan-task-attachments-count" aria-label="${items.length} attachments">${items.length}</span>`
    : '';
  const emptyHtml = items.length
    ? ''
    : `<span class="plan-task-attachments-empty">${escHtml(ATTACHMENTS_EMPTY_MSG)}</span>`;
  const listHtml = items.length
    ? `<ul class="plan-task-attachment-list" role="list">${items
        .map((entry) => {
          const added = formatRelativeTime(entry.added_at);
          const meta = added ? `Added ${added}` : 'Markdown attachment';
          return `
        <li
          class="plan-task-attachment-item"
          data-action="open-attachment"
          data-file-name="${escHtml(entry.file_name)}"
          role="button"
          tabindex="0"
          aria-label="Open attachment ${escHtml(entry.file_name)}"
        >
          <span class="plan-task-attachment-icon" aria-hidden="true">MD</span>
          <span class="plan-task-attachment-main">
            <span class="plan-task-attachment-name">${escHtml(entry.file_name)}</span>
            <span class="plan-task-attachment-meta">${escHtml(meta)}</span>
          </span>
          <span class="plan-task-attachment-open-hint" aria-hidden="true">Open</span>
          <button
            type="button"
            class="plan-task-attachment-delete"
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
    ? `<p class="plan-task-attachments-error" role="alert">${escHtml(ui.attachmentsError)}</p>`
    : '';
  const confirmHtml = confirmFile
    ? `
      <div
        class="plan-task-attachment-delete-confirm"
        data-attachment-delete-confirm
        role="dialog"
        aria-modal="true"
        aria-label="Delete attachment confirmation"
      >
        <div class="plan-task-attachment-delete-confirm-backdrop" data-action="cancel-delete-attachment"></div>
        <div class="plan-task-attachment-delete-confirm-panel">
          <h4 class="plan-task-attachment-delete-confirm-title">Delete attachment</h4>
          <p class="plan-task-attachment-delete-confirm-body">Delete “${escHtml(confirmFile)}”? The list entry and file will both be removed.</p>
          <div class="plan-task-attachment-delete-confirm-actions">
            <button
              type="button"
              class="md-header-btn"
              data-action="cancel-delete-attachment"
              ${disabledAttr}
            >Cancel</button>
            <button
              type="button"
              class="md-header-btn plan-task-btn-danger"
              data-action="confirm-delete-attachment"
              data-file-name="${escHtml(confirmFile)}"
              ${disabledAttr}
            >Confirm delete</button>
          </div>
        </div>
      </div>`
    : '';
  return `
    <section class="plan-task-attachments-section" aria-label="Attachments">
      <div class="plan-task-attachments-header">
        <div class="plan-task-attachments-heading">
          <h3 class="plan-task-attachments-title">Attachments</h3>
          ${countHtml}
        </div>
        <div class="plan-task-attachments-header-actions">
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

function renderCommentsSection(ui) {
  const disabledAttr = ui.disabled ? ' disabled' : '';
  const items = ui.comments ?? [];
  const editId = ui.commentEditId || '';
  const confirmId = ui.commentDeleteConfirm || '';
  const countHtml = items.length
    ? `<span class="plan-task-comments-count" aria-label="${items.length} process notes">${items.length}</span>`
    : '';
  const emptyHtml =
    !items.length && !ui.commentsError
      ? `<span class="plan-task-comments-empty">${escHtml(COMMENTS_EMPTY_MSG)}</span>`
      : '';
  const listHtml = items.length
    ? `<ul class="plan-task-comment-list" role="list">${items
        .map((entry) => {
          const added = formatRelativeTime(entry.created_at);
          const meta = added ? `Added ${added}` : 'Process note';
          if (editId === entry.id) {
            return `
        <li class="plan-task-comment-item plan-task-comment-item--editing" data-comment-id="${escHtml(entry.id)}">
          <textarea
            class="plan-task-comment-edit-area"
            data-comment-edit-input
            spellcheck="true"
            ${disabledAttr}
          >${escHtml(entry.body ?? '')}</textarea>
          <div class="plan-task-comment-item-actions">
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
        <li class="plan-task-comment-item" data-comment-id="${escHtml(entry.id)}">
          <div class="plan-task-comment-main">
            <p class="plan-task-comment-body">${escHtml(entry.body ?? '')}</p>
            <span class="plan-task-comment-meta">${escHtml(meta)}</span>
          </div>
          <div class="plan-task-comment-item-actions">
            <button
              type="button"
              class="md-header-btn"
              data-action="edit-comment"
              data-comment-id="${escHtml(entry.id)}"
              ${disabledAttr}
            >Edit</button>
            <button
              type="button"
              class="plan-task-comment-delete"
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
    ? `<p class="plan-task-comments-error" role="alert">${escHtml(ui.commentsError)}</p>`
    : '';
  const confirmHtml = confirmId
    ? `
      <div
        class="plan-task-comment-delete-confirm"
        data-comment-delete-confirm
        role="dialog"
        aria-modal="true"
        aria-label="Delete process note confirmation"
      >
        <div class="plan-task-comment-delete-confirm-backdrop" data-action="cancel-delete-comment"></div>
        <div class="plan-task-comment-delete-confirm-panel">
          <h4 class="plan-task-comment-delete-confirm-title">Delete process note</h4>
          <p class="plan-task-comment-delete-confirm-body">Delete this process note? This cannot be undone.</p>
          <div class="plan-task-comment-delete-confirm-actions">
            <button
              type="button"
              class="md-header-btn"
              data-action="cancel-delete-comment"
              ${disabledAttr}
            >Cancel</button>
            <button
              type="button"
              class="md-header-btn plan-task-btn-danger"
              data-action="confirm-delete-comment"
              data-comment-id="${escHtml(confirmId)}"
              ${disabledAttr}
            >Confirm delete</button>
          </div>
        </div>
      </div>`
    : '';
  return `
    <section class="plan-task-comments-section" aria-label="Process notes">
      <div class="plan-task-comments-header">
        <div class="plan-task-comments-heading">
          <h3 class="plan-task-comments-title">Process notes</h3>
          ${countHtml}
        </div>
        ${emptyHtml}
      </div>
      ${errHtml}
      ${listHtml}
      <div class="plan-task-comments-composer">
        <label class="plan-task-comments-composer-label" for="plan-task-comment-input">New process note</label>
        <textarea
          id="plan-task-comment-input"
          class="plan-task-comment-input"
          data-comment-input
          rows="3"
          placeholder="Add a process note…"
          spellcheck="true"
          ${disabledAttr}
        ></textarea>
        <div class="plan-task-comments-composer-actions">
          <button type="button" class="md-header-btn primary" data-action="add-comment"${disabledAttr}>Add note</button>
        </div>
      </div>
      ${confirmHtml}
    </section>
  `;
}

/**
 * Viewer-style attachment editor modal: preview by default, edit on demand.
 * @param {{ fileName: string, content: string, editMode: boolean, error: string, loading: boolean }} editor
 * @param {boolean} disabled
 */
function renderAttachmentEditor(editor, disabled) {
  const disabledAttr = disabled ? ' disabled' : '';
  const modeLabel = editor.loading ? 'Loading' : editor.editMode ? 'Edit' : 'Preview';
  const panelMode = editor.loading
    ? 'loading'
    : editor.editMode
      ? 'edit'
      : 'preview';
  const errHtml = editor.error
    ? `<p class="plan-task-attachment-error" role="alert">${escHtml(editor.error)}</p>`
    : '';
  let bodyHtml;
  let footerHtml = '';
  if (editor.loading) {
    bodyHtml = `
      <div class="plan-task-attachment-loading" role="status">
        <span class="plan-task-attachment-loading-dot" aria-hidden="true"></span>
        <span>Loading attachment…</span>
      </div>`;
  } else if (editor.editMode) {
    bodyHtml = `
      ${errHtml}
      <label class="plan-task-attachment-edit-label" for="plan-task-attachment-edit-area">Markdown source</label>
      <textarea
        id="plan-task-attachment-edit-area"
        class="plan-task-attachment-edit-area"
        spellcheck="false"
        ${disabledAttr}
      >${escHtml(editor.content ?? '')}</textarea>
    `;
    footerHtml = `
      <footer class="plan-task-attachment-editor-footer">
        <span class="plan-task-attachment-editor-footer-hint">Save will overwrite this attachment</span>
        <div class="plan-task-attachment-toolbar">
          <button type="button" class="md-header-btn" data-action="cancel-attachment-edit"${disabledAttr}>Cancel</button>
          <button type="button" class="md-header-btn primary" data-action="save-attachment"${disabledAttr}>Save</button>
        </div>
      </footer>`;
  } else {
    const previewHtml = editor.content
      ? renderCommentMarkdown(editor.content)
      : '<p class="plan-task-attachment-empty">No content</p>';
    bodyHtml = `
      ${errHtml}
      <article class="plan-task-attachment-doc">
        <div class="plan-task-attachment-preview">${previewHtml}</div>
      </article>
    `;
  }
  const editBtn =
    !editor.loading && !editor.editMode
      ? `<button type="button" class="md-header-btn primary" data-action="edit-attachment"${disabledAttr}>Edit</button>`
      : '';
  return `
    <div class="plan-task-attachment-editor" role="dialog" aria-modal="true" aria-label="${escHtml(editor.fileName)}">
      <div class="plan-task-attachment-editor-backdrop" data-action="close-attachment-editor"></div>
      <div class="plan-task-attachment-editor-panel plan-task-attachment-editor-panel--${panelMode}">
        <header class="plan-task-attachment-editor-header">
          <div class="plan-task-attachment-editor-heading">
            <span class="plan-task-attachment-icon" aria-hidden="true">MD</span>
            <div class="plan-task-attachment-editor-title-wrap">
              <h3 class="plan-task-attachment-editor-title">${escHtml(editor.fileName)}</h3>
              <span class="plan-task-attachment-editor-mode">${escHtml(modeLabel)}</span>
            </div>
          </div>
          <div class="plan-task-attachment-editor-actions">
            ${editBtn}
            <button type="button" class="md-header-btn" data-action="close-attachment-editor"${disabledAttr}>Close</button>
          </div>
        </header>
        <div class="plan-task-attachment-editor-body plan-task-attachment-editor-body--${panelMode}">${bodyHtml}</div>
        ${footerHtml}
      </div>
    </div>
  `;
}

function renderSubDetailPane(master, selectedSubId, ui) {
  const subs = master.sub_tasks ?? [];
  const items = subs.length
    ? subs.map((sub) => renderSubRow(master, sub, selectedSubId, ui)).join('')
    : renderSubEmpty(ui.disabled);
  return `
    <div class="plan-task-detail-body">
      <div class="plan-task-detail-header">
        <div>
          ${renderDetailTitle(master, ui)}
          ${renderDetailMeta(master)}
        </div>
      </div>
      ${master.migration_error ? renderMigrationWarning() : ''}
      ${renderPlanMdSection(master, ui)}
      ${renderAttachmentsSection(ui)}
      ${renderCommentsSection(ui)}
      ${renderRefreshWarning(ui.refreshWarning, ui.disabled)}
      ${renderDetailToolbar(master.master_task_id, ui.disabled)}
      <div class="plan-task-sub-list">${items}</div>
    </div>
  `;
}

function renderDetailEmpty() {
  return `
    <div class="plan-task-split-detail-empty plan-task-empty">
      <p class="plan-task-empty-title">Select a todo on the left</p>
      <p class="plan-task-empty-detail">Or create a todo from the top right</p>
    </div>
  `;
}

function renderDeadLink() {
  return `
    <div class="plan-task-split-dead-link plan-task-split-state">
      <p class="plan-task-split-state-title">Task not found</p>
      <p class="plan-task-split-state-detail">Link may be stale — pick again from the list</p>
    </div>
  `;
}

function renderErrorEmpty(message = UNAVAILABLE_MSG) {
  return `
    <div class="plan-task-split-error plan-task-split-state plan-task-split-state--error">
      <p class="plan-task-split-state-title">Temporarily unavailable</p>
      <p class="plan-task-split-state-detail">${escHtml(message)}</p>
    </div>
  `;
}

function renderPageShell({ masterHtml, detailHtml, disabled = false, activeOnly = true }) {
  return `
    <div class="plan-tasks-page">
      ${renderPageHeader(disabled, activeOnly)}
      <div class="plan-task-split">
        <aside class="plan-task-split-master" aria-label="Todos list">${masterHtml}</aside>
        <section class="plan-task-split-detail" aria-label="Task details">${detailHtml}</section>
      </div>
    </div>
  `;
}

function bindFocusRefresh(refresh) {
  const onFocus = () => {
    void refresh();
  };
  const onVisibility = () => {
    if (document.visibilityState === 'visible') {
      void refresh();
    }
  };
  window.addEventListener('focus', onFocus);
  document.addEventListener('visibilitychange', onVisibility);
  return () => {
    window.removeEventListener('focus', onFocus);
    document.removeEventListener('visibilitychange', onVisibility);
  };
}

/**
 * @param {HTMLElement} container
 * @param {{ masterId?: string, subId?: string, navigate?: (hash: string) => void }} [opts]
 */
export function mountPlanTaskSplit(container, opts = {}) {
  const { masterId: initialMasterId = '', subId: initialSubId = '', navigate } = opts;
  let disposed = false;
  let masters = [];
  let selectedMasterId = initialMasterId;
  let selectedSubId = initialSubId;
  let deadLink = false;
  let validateInitialSubLink = Boolean(initialSubId);
  let refreshPromise = null;
  let busy = false;
  let refreshWarning = '';
  let planMdEditMode = false;
  let planMdDraft = '';
  let planMdError = '';
  let planMdLoading = false;
  let attachments = [];
  let attachmentsError = '';
  /** @type {string} pending single-attachment delete confirm file name */
  let attachmentDeleteConfirm = '';
  /** @type {{ fileName: string, content: string, editMode: boolean, error: string, loading: boolean } | null} */
  let attachmentEditor = null;
  /** Bumped to ignore stale open/load results after close or newer open. */
  let attachmentLoadToken = 0;
  let comments = [];
  let commentsError = '';
  /** @type {string} comment id currently being edited */
  let commentEditId = '';
  /** @type {string} pending single-comment delete confirm id */
  let commentDeleteConfirm = '';
  const optimisticSubStatus = {};
  const subActionErrors = {};
  const subTitleErrors = {};
  const subTitleDrafts = {};
  let masterTitleDraft = '';
  let masterTitleError = '';
  /** Last master id painted into the detail pane — used to keep detail scroll across same-master paints. */
  let paintedMasterId = '';
  let activeOnly = true;
  /** Page lifecycle Binding: enter/select Set, leave Reset; shell close ≠ Reset. */
  const todosLifecycle = createTodosPageLifecycle();
  let lifecycleEntered = false;

  container.innerHTML = '<div class="plan-task-split-loading">Loading…</div>';

  function syncTodosBindingForSelection(masterId) {
    if (disposed) return;
    const id = typeof masterId === 'string' ? masterId.trim() : '';
    if (!lifecycleEntered) {
      lifecycleEntered = true;
      void todosLifecycle.onTodosPageEnter(id);
      return;
    }
    if (!id) return;
    void todosLifecycle.onMasterSelectionChange(id);
  }

  function findMaster(id) {
    return masters.find((master) => master.master_task_id === id) ?? null;
  }

  function getUi() {
    return {
      disabled: controlsDisabled(busy),
      refreshWarning,
      planMdEditMode,
      planMdDraft,
      planMdError,
      planMdLoading,
      attachments,
      attachmentsError,
      attachmentDeleteConfirm,
      comments,
      commentsError,
      commentEditId,
      commentDeleteConfirm,
      subStatus: optimisticSubStatus,
      subActionErrors,
      subTitleErrors,
      subTitleDrafts,
      masterTitleDraft: masterTitleDraft || undefined,
      masterTitleError,
    };
  }

  function resetPlanMdEdit() {
    planMdEditMode = false;
    planMdDraft = '';
    planMdError = '';
    planMdLoading = false;
  }

  function clearSubTitleState() {
    for (const key of Object.keys(subTitleDrafts)) {
      delete subTitleDrafts[key];
    }
    for (const key of Object.keys(subTitleErrors)) {
      delete subTitleErrors[key];
    }
  }

  function clearSubActionState() {
    for (const key of Object.keys(optimisticSubStatus)) {
      delete optimisticSubStatus[key];
    }
    for (const key of Object.keys(subActionErrors)) {
      delete subActionErrors[key];
    }
  }

  function closeSubMenus() {
    container.querySelectorAll('.plan-task-sub-menu-panel').forEach((panel) => {
      panel.hidden = true;
    });
  }

  function enforceActiveOnlySelection() {
    if (!selectedMasterId || !activeOnly) return;
    const master = findMaster(selectedMasterId);
    if (!master) return;
    if (!isIncompleteMaster(master)) {
      selectedMasterId = '';
      selectedSubId = '';
      deadLink = false;
      masterTitleDraft = '';
      masterTitleError = '';
      attachments = [];
      attachmentsError = '';
      comments = [];
      commentsError = '';
    }
  }

  function resolveSelection() {
    deadLink = false;
    if (!selectedMasterId) {
      selectedSubId = '';
    } else {
      const master = findMaster(selectedMasterId);
      if (!master) {
        deadLink = true;
        selectedSubId = '';
      } else {
        const subs = master.sub_tasks ?? [];
        if (validateInitialSubLink && initialSubId && selectedSubId === initialSubId) {
          validateInitialSubLink = false;
          const sub = subs.find((item) => item.sub_task_id === selectedSubId);
          if (!sub) {
            deadLink = true;
            selectedSubId = '';
          }
        } else if (
          !selectedSubId ||
          !subs.some((item) => item.sub_task_id === selectedSubId)
        ) {
          const fallback = pickDefaultSub(master);
          selectedSubId = fallback?.sub_task_id ?? '';
        }
      }
    }
    enforceActiveOnlySelection();
  }

  function renderDetailPane() {
    if (deadLink) return renderDeadLink();
    if (!selectedMasterId) return renderDetailEmpty();
    const master = findMaster(selectedMasterId);
    if (!master) return renderDeadLink();
    return renderSubDetailPane(master, selectedSubId, getUi());
  }

  function setAttachmentEditor(next) {
    attachmentEditor = next;
  }

  /** Refresh an existing editor node in place so callers holding a DOM ref stay valid. */
  function refreshEditorNode(editorNode, ui) {
    const tmp = document.createElement('div');
    tmp.innerHTML = renderAttachmentEditor(attachmentEditor, ui.disabled);
    const fresh = tmp.firstElementChild;
    if (!fresh) return false;
    editorNode.innerHTML = fresh.innerHTML;
    for (const attr of [...fresh.attributes]) {
      editorNode.setAttribute(attr.name, attr.value);
    }
    return true;
  }

  function paint() {
    if (!masters.length && container.querySelector('.plan-task-split-error')) {
      return;
    }
    const masterPane = container.querySelector('.plan-task-split-master');
    const detailPane = container.querySelector('.plan-task-split-detail');
    const masterScroll = masterPane?.scrollTop ?? 0;
    const detailScroll = detailPane?.scrollTop ?? 0;
    const keepDetailScroll =
      Boolean(selectedMasterId) && selectedMasterId === paintedMasterId;
    const ui = getUi();
    const masterHtml = renderMasterPane(masters, selectedMasterId, ui.disabled, activeOnly);
    // Keep the same editor element node across paints so in-flight UI refs stay valid.
    const existingEditor = container.querySelector('.plan-task-attachment-editor');
    if (existingEditor) existingEditor.remove();
    container.innerHTML = renderPageShell({
      masterHtml,
      detailHtml: renderDetailPane(),
      disabled: ui.disabled,
      activeOnly,
    });
    paintedMasterId = selectedMasterId;
    const nextMaster = container.querySelector('.plan-task-split-master');
    const nextDetail = container.querySelector('.plan-task-split-detail');
    if (nextMaster) nextMaster.scrollTop = masterScroll;
    if (nextDetail && keepDetailScroll) nextDetail.scrollTop = detailScroll;
    if (!attachmentEditor) return;
    if (existingEditor && refreshEditorNode(existingEditor, ui)) {
      container.appendChild(existingEditor);
      return;
    }
    container.insertAdjacentHTML(
      'beforeend',
      renderAttachmentEditor(attachmentEditor, ui.disabled),
    );
  }

  /**
   * In-place deep-link update (hash change while already on plan-tasks).
   * No-ops when selection already matches so click→navigate does not double-load.
   * @param {{ masterId?: string, subId?: string }} [route]
   */
  function applyRoute(route = {}) {
    if (disposed) return;
    const masterId = route.masterId ?? '';
    const subId = route.subId ?? '';
    if (masterId === selectedMasterId && subId === selectedSubId) return;
    const masterChanged = masterId !== selectedMasterId;
    closeSubMenus();
    resetPlanMdEdit();
    closeAttachmentEditor();
    clearSubActionState();
    clearSubTitleState();
    masterTitleDraft = '';
    masterTitleError = '';
    selectedMasterId = masterId;
    selectedSubId = subId;
    validateInitialSubLink = Boolean(subId);
    deadLink = false;
    attachments = [];
    attachmentsError = '';
    attachmentDeleteConfirm = '';
    comments = [];
    commentsError = '';
    commentEditId = '';
    commentDeleteConfirm = '';
    resolveSelection();
    if (masterChanged) {
      syncTodosBindingForSelection(selectedMasterId);
    }
    void (async () => {
      await loadAttachmentsForSelected();
      await loadCommentsForSelected();
      if (disposed) return;
      paint();
    })();
  }

  function closeAttachmentEditor() {
    attachmentLoadToken += 1;
    setAttachmentEditor(null);
  }

  async function openAttachmentEditor(fileName) {
    if (!selectedMasterId || controlsDisabled(busy) || !fileName) return;
    const masterTaskId = selectedMasterId;
    const loadToken = ++attachmentLoadToken;
    setAttachmentEditor({
      fileName,
      content: '',
      editMode: false,
      error: '',
      loading: true,
    });
    busy = true;
    paint();
    try {
      const result = await readPlanAttachment({ masterTaskId, fileName });
      if (
        disposed ||
        selectedMasterId !== masterTaskId ||
        loadToken !== attachmentLoadToken
      ) {
        return;
      }
      const content =
        typeof result === 'string' ? result : String(result?.content ?? '');
      setAttachmentEditor({
        fileName,
        content,
        editMode: false,
        error: '',
        loading: false,
      });
    } catch (err) {
      if (
        disposed ||
        selectedMasterId !== masterTaskId ||
        loadToken !== attachmentLoadToken
      ) {
        return;
      }
      setAttachmentEditor({
        fileName,
        content: '',
        editMode: false,
        error: err?.message || 'Failed to load attachment',
        loading: false,
      });
    } finally {
      if (loadToken === attachmentLoadToken) {
        busy = false;
        if (!disposed) paint();
      }
    }
  }

  function enterAttachmentEditMode() {
    if (!attachmentEditor || attachmentEditor.loading || controlsDisabled(busy)) return;
    setAttachmentEditor({ ...attachmentEditor, editMode: true, error: '' });
    paint();
  }

  function cancelAttachmentEdit() {
    if (!attachmentEditor) return;
    setAttachmentEditor({ ...attachmentEditor, editMode: false, error: '' });
    paint();
  }

  async function saveAttachmentEditor() {
    if (!attachmentEditor || !selectedMasterId || controlsDisabled(busy)) return;
    const fileName = attachmentEditor.fileName;
    const editorEl = container.querySelector('.plan-task-attachment-edit-area');
    const content =
      editorEl instanceof HTMLTextAreaElement
        ? editorEl.value
        : attachmentEditor.content;
    setAttachmentEditor({ ...attachmentEditor, content, error: '' });
    busy = true;
    paint();
    try {
      await savePlanAttachment({
        masterTaskId: selectedMasterId,
        fileName,
        content,
      });
      if (disposed) return;
      setAttachmentEditor({
        fileName,
        content,
        editMode: false,
        error: '',
        loading: false,
      });
    } catch (err) {
      if (disposed) return;
      setAttachmentEditor({
        fileName,
        content,
        editMode: true,
        error: err?.message || 'Save failed',
        loading: false,
      });
    } finally {
      busy = false;
      if (!disposed) paint();
    }
  }

  async function loadAttachmentsForSelected() {
    if (!selectedMasterId) {
      attachments = [];
      return;
    }
    const masterId = selectedMasterId;
    try {
      const entries = await listPlanAttachments({ masterTaskId: masterId });
      if (disposed || selectedMasterId !== masterId) return;
      attachments = entries;
    } catch {
      if (disposed || selectedMasterId !== masterId) return;
      attachments = [];
    }
  }

  async function loadCommentsForSelected() {
    if (!selectedMasterId) {
      comments = [];
      commentsError = '';
      return;
    }
    const masterId = selectedMasterId;
    try {
      const entries = await listPlanComments({ masterTaskId: masterId });
      if (disposed || selectedMasterId !== masterId) return;
      comments = entries;
      commentsError = '';
    } catch (err) {
      if (disposed || selectedMasterId !== masterId) return;
      comments = [];
      commentsError = err?.message || 'Failed to load process notes';
    }
  }

  async function reloadList({ afterWrite = false } = {}) {
    try {
      const entries = await loadPlanTasks();
      if (disposed) return;
      masters = entries;
      resolveSelection();
      if (afterWrite) {
        refreshWarning = '';
      }
      await loadAttachmentsForSelected();
      await loadCommentsForSelected();
      if (disposed) return;
      paint();
      if (!lifecycleEntered) {
        syncTodosBindingForSelection(selectedMasterId);
      }
    } catch {
      if (disposed) return;
      if (afterWrite) {
        refreshWarning = REFRESH_WARNING_MSG;
        paint();
        return;
      }
      masters = [];
      selectedMasterId = '';
      selectedSubId = '';
      attachments = [];
      attachmentsError = '';
      comments = [];
      commentsError = '';
      commentEditId = '';
      commentDeleteConfirm = '';
      refreshWarning = '';
      container.innerHTML = renderPageShell({
        masterHtml: '<div class="plan-task-split-state"></div>',
        detailHtml: renderErrorEmpty(),
      });
      if (!lifecycleEntered) {
        syncTodosBindingForSelection('');
      }
    }
  }

  async function pickAndAddAttachment() {
    if (!selectedMasterId || controlsDisabled(busy)) return;
    attachmentsError = '';
    paint();
    let picked;
    try {
      picked = await pickLocalMarkdownFile();
    } catch (err) {
      attachmentsError = err?.message
        ? `Failed to pick file: ${err.message}`
        : 'Failed to pick file';
      paint();
      return;
    }
    if (!picked) {
      attachmentsError = ATTACHMENT_PICK_CANCEL_MSG;
      paint();
      return;
    }
    busy = true;
    paint();
    try {
      await addPlanAttachment({
        masterTaskId: selectedMasterId,
        fileName: picked.fileName,
        content: picked.content,
      });
      attachmentsError = '';
      busy = false;
      await loadAttachmentsForSelected();
      if (!disposed) paint();
    } catch (err) {
      busy = false;
      attachmentsError = err?.message || 'Failed to add attachment';
      paint();
    }
  }

  function openAttachmentDeleteConfirm(fileName) {
    if (!selectedMasterId || controlsDisabled(busy) || !fileName) return;
    attachmentDeleteConfirm = fileName;
    attachmentsError = '';
    paint();
  }

  function cancelAttachmentDeleteConfirm() {
    attachmentDeleteConfirm = '';
    paint();
  }

  async function confirmAndDeleteAttachment(fileName) {
    if (!selectedMasterId || controlsDisabled(busy) || !fileName) return;
    const masterTaskId = selectedMasterId;
    busy = true;
    attachmentsError = '';
    paint();
    try {
      await deletePlanAttachment({ masterTaskId, fileName });
      if (disposed || selectedMasterId !== masterTaskId) return;
      attachmentDeleteConfirm = '';
      attachmentsError = '';
      await loadAttachmentsForSelected();
    } catch (err) {
      if (disposed || selectedMasterId !== masterTaskId) return;
      attachmentDeleteConfirm = '';
      attachmentsError = err?.message || 'Failed to delete attachment';
    } finally {
      busy = false;
      if (!disposed) paint();
    }
  }

  async function addCommentFromComposer() {
    if (!selectedMasterId || controlsDisabled(busy)) return;
    const input = container.querySelector('[data-comment-input]');
    const body = input instanceof HTMLTextAreaElement ? input.value : '';
    const masterTaskId = selectedMasterId;
    busy = true;
    commentsError = '';
    paint();
    try {
      await addPlanComment({ masterTaskId, body });
      if (disposed || selectedMasterId !== masterTaskId) return;
      commentsError = '';
      commentEditId = '';
      await loadCommentsForSelected();
    } catch (err) {
      if (disposed || selectedMasterId !== masterTaskId) return;
      commentsError = err?.message || 'Failed to add process note';
    } finally {
      busy = false;
      if (!disposed) paint();
    }
  }

  function beginCommentEdit(commentId) {
    if (!selectedMasterId || controlsDisabled(busy) || !commentId) return;
    commentEditId = commentId;
    commentDeleteConfirm = '';
    commentsError = '';
    paint();
  }

  function cancelCommentEdit() {
    commentEditId = '';
    paint();
  }

  async function saveCommentEdit(commentId) {
    if (!selectedMasterId || controlsDisabled(busy) || !commentId) return;
    const editInput = container.querySelector('[data-comment-edit-input]');
    const body = editInput instanceof HTMLTextAreaElement ? editInput.value : '';
    const masterTaskId = selectedMasterId;
    busy = true;
    commentsError = '';
    paint();
    try {
      await updatePlanComment({ masterTaskId, commentId, body });
      if (disposed || selectedMasterId !== masterTaskId) return;
      commentEditId = '';
      commentsError = '';
      await loadCommentsForSelected();
    } catch (err) {
      if (disposed || selectedMasterId !== masterTaskId) return;
      commentsError = err?.message || 'Failed to update process note';
    } finally {
      busy = false;
      if (!disposed) paint();
    }
  }

  function openCommentDeleteConfirm(commentId) {
    if (!selectedMasterId || controlsDisabled(busy) || !commentId) return;
    commentDeleteConfirm = commentId;
    commentEditId = '';
    commentsError = '';
    paint();
  }

  function cancelCommentDeleteConfirm() {
    commentDeleteConfirm = '';
    paint();
  }

  async function confirmAndDeleteComment(commentId) {
    if (!selectedMasterId || controlsDisabled(busy) || !commentId) return;
    const masterTaskId = selectedMasterId;
    busy = true;
    commentsError = '';
    paint();
    try {
      await deletePlanComment({ masterTaskId, commentId });
      if (disposed || selectedMasterId !== masterTaskId) return;
      commentDeleteConfirm = '';
      commentsError = '';
      await loadCommentsForSelected();
    } catch (err) {
      if (disposed || selectedMasterId !== masterTaskId) return;
      commentDeleteConfirm = '';
      commentsError = err?.message || 'Failed to delete process note';
    } finally {
      busy = false;
      if (!disposed) paint();
    }
  }

  async function refresh() {
    if (refreshPromise) {
      return refreshPromise;
    }
    refreshPromise = reloadList().finally(() => {
      refreshPromise = null;
    });
    return refreshPromise;
  }

  async function runWriteAction(actionFn) {
    busy = true;
    paint();
    try {
      await actionFn();
      busy = false;
      await reloadList({ afterWrite: true });
    } catch (err) {
      busy = false;
      throw err;
    }
  }

  async function enterPlanMdEdit() {
    if (!selectedMasterId || controlsDisabled(busy)) return;
    planMdLoading = true;
    planMdError = '';
    paint();
    try {
      planMdDraft = await readPlanMd({ masterTaskId: selectedMasterId });
      planMdEditMode = true;
    } catch (err) {
      planMdError = err?.message || 'Failed to load description';
      planMdEditMode = false;
    } finally {
      planMdLoading = false;
      paint();
    }
  }

  async function savePlanMd() {
    if (!selectedMasterId || controlsDisabled(busy)) return;
    const editor = container.querySelector('.plan-task-plan-md-editor');
    planMdDraft = editor instanceof HTMLTextAreaElement ? editor.value : planMdDraft;
    planMdError = '';
    busy = true;
    paint();
    try {
      await updatePlanMd({ masterTaskId: selectedMasterId, planMd: planMdDraft });
      resetPlanMdEdit();
      busy = false;
      await reloadList({ afterWrite: true });
    } catch (err) {
      busy = false;
      planMdEditMode = true;
      planMdError = err?.message || 'Save failed';
      paint();
    }
  }

  function cancelPlanMdEdit() {
    resetPlanMdEdit();
    paint();
  }

  async function runSubStatusAction(subTaskId, targetStatus, actionFn) {
    const master = findMaster(selectedMasterId);
    const sub = master?.sub_tasks?.find((item) => item.sub_task_id === subTaskId);
    if (!master || !sub || sub.status !== 'incomplete') return;
    delete subActionErrors[subTaskId];
    optimisticSubStatus[subTaskId] = targetStatus;
    busy = true;
    paint();
    try {
      await actionFn({ masterTaskId: selectedMasterId, subTaskId });
      delete optimisticSubStatus[subTaskId];
      busy = false;
      await reloadList({ afterWrite: true });
    } catch (err) {
      delete optimisticSubStatus[subTaskId];
      busy = false;
      subActionErrors[subTaskId] = err?.message || 'Operation failed';
      paint();
    }
  }

  async function runSubTitleSave(subTaskId, title) {
    const master = findMaster(selectedMasterId);
    const sub = master?.sub_tasks?.find((item) => item.sub_task_id === subTaskId);
    if (!master || !sub) return;
    const trimmed = title.trim();
    if (!trimmed) {
      subTitleErrors[subTaskId] = 'Title cannot be empty';
      paint();
      return;
    }
    if (trimmed === (sub.title || sub.sub_task_id)) {
      delete subTitleDrafts[subTaskId];
      delete subTitleErrors[subTaskId];
      return;
    }
    delete subTitleErrors[subTaskId];
    subTitleDrafts[subTaskId] = trimmed;
    busy = true;
    paint();
    try {
      await updatePlanSub({ masterTaskId: selectedMasterId, subTaskId, title: trimmed });
      delete subTitleDrafts[subTaskId];
      busy = false;
      await reloadList({ afterWrite: true });
    } catch (err) {
      delete subTitleDrafts[subTaskId];
      busy = false;
      subTitleErrors[subTaskId] = err?.message || 'Save failed';
      paint();
    }
  }

  async function runMasterTitleSave(title) {
    const master = findMaster(selectedMasterId);
    if (!master) return;
    const trimmed = title.trim();
    if (!trimmed) {
      masterTitleError = 'Title cannot be empty';
      paint();
      return;
    }
    if (trimmed === master.title) {
      masterTitleDraft = '';
      masterTitleError = '';
      return;
    }
    masterTitleError = '';
    masterTitleDraft = trimmed;
    busy = true;
    paint();
    try {
      await updatePlanMasterTitle({ masterTaskId: selectedMasterId, title: trimmed });
      masterTitleDraft = '';
      busy = false;
      await reloadList({ afterWrite: true });
    } catch (err) {
      busy = false;
      masterTitleError = err?.message || 'Save failed';
      paint();
    }
  }

  async function runSubStatusChange(subTaskId, targetStatus, selectEl) {
    const master = findMaster(selectedMasterId);
    const sub = master?.sub_tasks?.find((item) => item.sub_task_id === subTaskId);
    const priorStatus = sub?.status ?? 'incomplete';
    if (!master || !sub || sub.status !== 'incomplete') {
      if (selectEl instanceof HTMLSelectElement) {
        selectEl.value = priorStatus;
      }
      return;
    }
    if (targetStatus === priorStatus) return;
    if (targetStatus !== 'complete' && targetStatus !== 'abandoned') {
      if (selectEl instanceof HTMLSelectElement) {
        selectEl.value = priorStatus;
      }
      return;
    }
    try {
      if (targetStatus === 'complete') {
        await runSubStatusAction(subTaskId, 'complete', completePlan);
      } else {
        await runSubStatusAction(subTaskId, 'abandoned', abandonPlanSub);
      }
    } catch {
      if (selectEl instanceof HTMLSelectElement) {
        selectEl.value = priorStatus;
      }
    }
  }

  async function runMasterStatusChange(targetStatus, selectEl) {
    const master = findMaster(selectedMasterId);
    const priorStatus = masterStatusClass(master?.status);
    if (!master) {
      if (selectEl instanceof HTMLSelectElement) {
        selectEl.value = priorStatus;
      }
      return;
    }
    if (
      targetStatus !== 'incomplete' &&
      targetStatus !== 'complete' &&
      targetStatus !== 'abandoned'
    ) {
      if (selectEl instanceof HTMLSelectElement) {
        selectEl.value = priorStatus;
      }
      return;
    }
    if (targetStatus === priorStatus) return;
    busy = true;
    paint();
    try {
      await setPlanMasterStatus({
        masterTaskId: selectedMasterId,
        status: targetStatus,
      });
      busy = false;
      await reloadList({ afterWrite: true });
    } catch {
      busy = false;
      if (selectEl instanceof HTMLSelectElement) {
        selectEl.value = priorStatus;
      }
      paint();
    }
  }

  /** Present-only: open/focus assistant shell. Does not Set or write bound_master_task_id. */
  async function presentTodosAssistant() {
    if (!selectedMasterId || controlsDisabled(busy)) return;
    const invoke = getTauriInvoke();
    if (!invoke) return;
    try {
      await invoke('present_ai_assistant');
    } catch {
      // Open/focus failure is non-fatal; Host returns explicit errors when busy.
    }
  }

  function onAiAssistantTurnCompleted(event) {
    if (disposed) return;
    const payload = event?.payload;
    if (!payload || payload.wrote !== true) return;
    void reloadList({ afterWrite: true });
  }

  function openCreateDialog(triggerEl) {
    openPlanTaskDialog({
      type: 'create-master',
      triggerEl,
      onSubmit: async ({ title, subTitles }) => {
        await runWriteAction(async () => {
          const result = await createPlanTask({ title, subTitles });
          const createdId = result?.master_task_id ?? result?.task?.master_task_id;
          if (createdId) {
            selectedMasterId = createdId;
            selectedSubId = '';
            deadLink = false;
            syncTodosBindingForSelection(selectedMasterId);
          }
        });
      },
    });
  }

  function openAddSubDialog(triggerEl) {
    const master = findMaster(selectedMasterId);
    if (!master) return;
    openPlanTaskDialog({
      type: 'add-sub',
      triggerEl,
      payload: { masterTitle: master.title },
      onSubmit: async ({ title }) => {
        const masterTaskId = selectedMasterId;
        await runWriteAction(async () => {
          await addPlanSub({ masterTaskId, title });
        });
      },
    });
  }

  function openDeleteMasterDialog(triggerEl) {
    const master = findMaster(selectedMasterId);
    if (!master) return;
    openPlanTaskDialog({
      type: 'delete-master',
      triggerEl,
      payload: {
        masterTitle: master.title,
        subCount: master.sub_tasks?.length ?? 0,
      },
      onSubmit: async () => {
        const masterTaskId = selectedMasterId;
        await runWriteAction(async () => {
          await deletePlanTask({ masterTaskId });
          selectedMasterId = '';
          selectedSubId = '';
          deadLink = false;
          if (typeof navigate === 'function') {
            navigate('#/plan-tasks');
          }
        });
      },
    });
  }

  function openDeleteSubDialog(triggerEl, subTaskId, subTitle) {
    openPlanTaskDialog({
      type: 'delete-sub',
      triggerEl,
      payload: { subTitle },
      onSubmit: async () => {
        const masterTaskId = selectedMasterId;
        const deletingSelected = selectedSubId === subTaskId;
        await runWriteAction(async () => {
          await deletePlanSub({ masterTaskId, subTaskId });
          if (deletingSelected) {
            selectedSubId = '';
            deadLink = false;
            if (typeof navigate === 'function' && masterTaskId) {
              navigate(buildMasterDeepLink(masterTaskId));
            }
          }
        });
      },
    });
  }

  const onClick = (event) => {
    const actionEl = event.target.closest('[data-action]');
    const action = actionEl?.dataset.action;

    if (action === 'retry-refresh') {
      event.preventDefault();
      if (controlsDisabled(busy)) return;
      refreshWarning = '';
      void refresh();
      return;
    }

    if (action === 'edit-plan-md') {
      event.preventDefault();
      if (controlsDisabled(busy)) return;
      void enterPlanMdEdit();
      return;
    }

    if (action === 'save-plan-md') {
      event.preventDefault();
      if (controlsDisabled(busy)) return;
      void savePlanMd();
      return;
    }

    if (action === 'cancel-plan-md') {
      event.preventDefault();
      if (controlsDisabled(busy)) return;
      cancelPlanMdEdit();
      return;
    }

    if (action === 'toggle-active-only') {
      event.preventDefault();
      if (controlsDisabled(busy)) return;
      activeOnly = !activeOnly;
      enforceActiveOnlySelection();
      paint();
      return;
    }

    if (action === 'create-master') {
      event.preventDefault();
      if (controlsDisabled(busy)) return;
      openCreateDialog(actionEl instanceof HTMLElement ? actionEl : null);
      return;
    }

    if (action === 'open-ai-assistant') {
      event.preventDefault();
      if (controlsDisabled(busy) || !selectedMasterId) return;
      void presentTodosAssistant();
      return;
    }

    if (action === 'pick-attachment-md') {
      event.preventDefault();
      if (controlsDisabled(busy) || !selectedMasterId) return;
      void pickAndAddAttachment();
      return;
    }

    if (action === 'delete-attachment') {
      event.preventDefault();
      event.stopPropagation();
      if (controlsDisabled(busy) || !selectedMasterId) return;
      const fileName = actionEl?.dataset.fileName;
      if (!fileName) return;
      openAttachmentDeleteConfirm(fileName);
      return;
    }

    if (action === 'confirm-delete-attachment') {
      event.preventDefault();
      event.stopPropagation();
      if (controlsDisabled(busy) || !selectedMasterId) return;
      const fileName = actionEl?.dataset.fileName || attachmentDeleteConfirm;
      if (!fileName) return;
      void confirmAndDeleteAttachment(fileName);
      return;
    }

    if (action === 'cancel-delete-attachment') {
      event.preventDefault();
      event.stopPropagation();
      cancelAttachmentDeleteConfirm();
      return;
    }

    if (action === 'add-comment') {
      event.preventDefault();
      if (controlsDisabled(busy) || !selectedMasterId) return;
      void addCommentFromComposer();
      return;
    }

    if (action === 'edit-comment') {
      event.preventDefault();
      if (controlsDisabled(busy) || !selectedMasterId) return;
      const commentId = actionEl?.dataset.commentId;
      if (!commentId) return;
      beginCommentEdit(commentId);
      return;
    }

    if (action === 'save-comment') {
      event.preventDefault();
      if (controlsDisabled(busy) || !selectedMasterId) return;
      const commentId = actionEl?.dataset.commentId || commentEditId;
      if (!commentId) return;
      void saveCommentEdit(commentId);
      return;
    }

    if (action === 'cancel-comment-edit') {
      event.preventDefault();
      if (controlsDisabled(busy)) return;
      cancelCommentEdit();
      return;
    }

    if (action === 'delete-comment') {
      event.preventDefault();
      if (controlsDisabled(busy) || !selectedMasterId) return;
      const commentId = actionEl?.dataset.commentId;
      if (!commentId) return;
      openCommentDeleteConfirm(commentId);
      return;
    }

    if (action === 'confirm-delete-comment') {
      event.preventDefault();
      event.stopPropagation();
      if (controlsDisabled(busy) || !selectedMasterId) return;
      const commentId = actionEl?.dataset.commentId || commentDeleteConfirm;
      if (!commentId) return;
      void confirmAndDeleteComment(commentId);
      return;
    }

    if (action === 'cancel-delete-comment') {
      event.preventDefault();
      event.stopPropagation();
      cancelCommentDeleteConfirm();
      return;
    }

    if (action === 'open-attachment') {
      event.preventDefault();
      if (controlsDisabled(busy) || !selectedMasterId) return;
      const fileName = actionEl?.dataset.fileName;
      if (!fileName) return;
      void openAttachmentEditor(fileName);
      return;
    }

    if (action === 'edit-attachment') {
      event.preventDefault();
      if (controlsDisabled(busy)) return;
      enterAttachmentEditMode();
      return;
    }

    if (action === 'save-attachment') {
      event.preventDefault();
      if (controlsDisabled(busy)) return;
      void saveAttachmentEditor();
      return;
    }

    if (action === 'cancel-attachment-edit') {
      event.preventDefault();
      if (controlsDisabled(busy)) return;
      cancelAttachmentEdit();
      return;
    }

    if (action === 'close-attachment-editor') {
      event.preventDefault();
      closeAttachmentEditor();
      paint();
      return;
    }

    if (action === 'add-sub') {
      event.preventDefault();
      if (controlsDisabled(busy) || !selectedMasterId) return;
      openAddSubDialog(actionEl instanceof HTMLElement ? actionEl : null);
      return;
    }

    if (action === 'delete-master') {
      event.preventDefault();
      if (controlsDisabled(busy) || !selectedMasterId) return;
      openDeleteMasterDialog(actionEl instanceof HTMLElement ? actionEl : null);
      return;
    }

    if (action === 'delete-sub') {
      event.preventDefault();
      event.stopPropagation();
      closeSubMenus();
      if (controlsDisabled(busy) || !selectedMasterId) return;
      const subTaskId = actionEl?.dataset.subId;
      const subTitle = actionEl?.dataset.subTitle ?? '';
      if (!subTaskId) return;
      openDeleteSubDialog(
        actionEl instanceof HTMLElement ? actionEl : null,
        subTaskId,
        subTitle,
      );
      return;
    }

    if (action === 'toggle-sub-menu') {
      event.preventDefault();
      event.stopPropagation();
      const panel = actionEl?.closest('.plan-task-sub-menu')?.querySelector('.plan-task-sub-menu-panel');
      if (!(panel instanceof HTMLElement)) return;
      const willOpen = panel.hidden;
      closeSubMenus();
      panel.hidden = !willOpen;
      return;
    }

    if (action === 'copy-sub-id' || action === 'copy-master-id') {
      event.preventDefault();
      event.stopPropagation();
      const text = actionEl?.dataset.copyText ?? '';
      if (text && navigator.clipboard?.writeText) {
        void navigator.clipboard.writeText(text).then(() => {
          if (action === 'copy-master-id' && actionEl instanceof HTMLElement) {
            flashCopyFeedback(actionEl, COPY_MASTER_ID_LABEL);
          }
        });
      }
      closeSubMenus();
      return;
    }

    const masterBtn = event.target.closest('.plan-task-master-item');
    if (masterBtn?.dataset.masterId) {
      if (controlsDisabled(busy)) return;
      // Same master: keep list/detail scroll and current sub selection.
      if (masterBtn.dataset.masterId === selectedMasterId) {
        closeSubMenus();
        return;
      }
      closeSubMenus();
      resetPlanMdEdit();
      closeAttachmentEditor();
      clearSubActionState();
      clearSubTitleState();
      masterTitleDraft = '';
      masterTitleError = '';
      selectedMasterId = masterBtn.dataset.masterId;
      const master = findMaster(selectedMasterId);
      const fallback = master ? pickDefaultSub(master) : null;
      selectedSubId = fallback?.sub_task_id ?? '';
      deadLink = false;
      attachments = [];
      attachmentsError = '';
      attachmentDeleteConfirm = '';
      comments = [];
      commentsError = '';
      commentEditId = '';
      commentDeleteConfirm = '';
      syncTodosBindingForSelection(selectedMasterId);
      void (async () => {
        await loadAttachmentsForSelected();
        await loadCommentsForSelected();
        if (disposed) return;
        paint();
        if (typeof navigate === 'function' && selectedMasterId && selectedSubId) {
          navigate(buildDeepLink(selectedMasterId, selectedSubId));
        }
      })();
      return;
    }

    const subEl = event.target.closest('.plan-task-sub');
    if (subEl?.dataset.subId && selectedMasterId) {
      if (controlsDisabled(busy)) return;
      if (event.target.closest('.plan-task-sub-menu')) return;
      if (event.target.closest('.plan-task-sub-title-input')) return;
      if (event.target.closest('.plan-task-sub-status-select')) return;
      closeSubMenus();
      selectedSubId = subEl.dataset.subId;
      deadLink = false;
      paint();
      if (typeof navigate === 'function') {
        navigate(buildDeepLink(selectedMasterId, selectedSubId));
      }
      return;
    }

    if (!event.target.closest('.plan-task-sub-menu')) {
      closeSubMenus();
    }
  };

  const onInput = (event) => {
    const masterTitleInput = event.target.closest('[data-action="edit-master-title"]');
    if (masterTitleInput instanceof HTMLInputElement) {
      if (controlsDisabled(busy) || !selectedMasterId) return;
      masterTitleDraft = masterTitleInput.value;
      return;
    }
    const input = event.target.closest('[data-action="edit-sub-title"]');
    if (!(input instanceof HTMLInputElement) || controlsDisabled(busy) || !selectedMasterId) return;
    subTitleDrafts[input.dataset.subId ?? ''] = input.value;
  };

  const onKeydown = (event) => {
    if (event.key === 'Escape') {
      if (commentDeleteConfirm) {
        event.preventDefault();
        cancelCommentDeleteConfirm();
        return;
      }
      if (commentEditId) {
        event.preventDefault();
        if (controlsDisabled(busy)) return;
        cancelCommentEdit();
        return;
      }
      if (attachmentDeleteConfirm) {
        event.preventDefault();
        cancelAttachmentDeleteConfirm();
        return;
      }
      if (attachmentEditor) {
        event.preventDefault();
        if (attachmentEditor.loading) {
          closeAttachmentEditor();
          busy = false;
          paint();
          return;
        }
        if (controlsDisabled(busy)) return;
        if (attachmentEditor.editMode) {
          cancelAttachmentEdit();
          return;
        }
        closeAttachmentEditor();
        paint();
        return;
      }
    }

    const attachItem = event.target.closest('[data-action="open-attachment"]');
    if (
      attachItem instanceof HTMLElement &&
      (event.key === 'Enter' || event.key === ' ')
    ) {
      event.preventDefault();
      if (controlsDisabled(busy) || !selectedMasterId) return;
      const fileName = attachItem.dataset.fileName;
      if (!fileName) return;
      void openAttachmentEditor(fileName);
      return;
    }

    const titleInput = event.target.closest(
      '[data-action="edit-master-title"], [data-action="edit-sub-title"]',
    );
    if (!(titleInput instanceof HTMLInputElement)) return;
    if (event.key === 'Enter') {
      event.preventDefault();
      titleInput.blur();
    }
  };

  const onTitleBlur = (event) => {
    const masterTitleInput = event.target.closest('[data-action="edit-master-title"]');
    if (masterTitleInput instanceof HTMLInputElement) {
      if (controlsDisabled(busy) || !selectedMasterId) return;
      void runMasterTitleSave(masterTitleInput.value);
      return;
    }
    const input = event.target.closest('[data-action="edit-sub-title"]');
    if (!(input instanceof HTMLInputElement) || controlsDisabled(busy) || !selectedMasterId) return;
    const subTaskId = input.dataset.subId;
    if (!subTaskId) return;
    void runSubTitleSave(subTaskId, input.value);
  };

  const onChange = (event) => {
    const masterSelect = event.target.closest('[data-action="change-master-status"]');
    if (masterSelect instanceof HTMLSelectElement) {
      if (controlsDisabled(busy) || !selectedMasterId) {
        const master = findMaster(selectedMasterId);
        masterSelect.value = masterStatusClass(master?.status);
        return;
      }
      void runMasterStatusChange(masterSelect.value, masterSelect);
      return;
    }
    const select = event.target.closest('[data-action="change-sub-status"]');
    if (!(select instanceof HTMLSelectElement)) return;
    if (controlsDisabled(busy) || !selectedMasterId) {
      select.value = select.dataset.currentStatus ?? select.value;
      return;
    }
    const subTaskId = select.dataset.subId;
    const targetStatus = select.value;
    if (!subTaskId) return;
    void runSubStatusChange(subTaskId, targetStatus, select);
  };

  container.addEventListener('click', onClick);
  container.addEventListener('input', onInput);
  container.addEventListener('keydown', onKeydown);
  container.addEventListener('focusout', onTitleBlur);
  container.addEventListener('change', onChange);
  const onDialogClose = () => {
    if (!disposed) paint();
  };
  document.addEventListener('plan-task-dialog-close', onDialogClose);
  const disposeFocusRefresh = bindFocusRefresh(refresh);

  let unlistenTurnCompleted = null;
  const listen = getTauriListen();
  if (listen) {
    void listen(AI_ASSISTANT_TURN_COMPLETED, onAiAssistantTurnCompleted).then(
      (unlisten) => {
        if (disposed) {
          if (typeof unlisten === 'function') void unlisten();
          return;
        }
        unlistenTurnCompleted = unlisten;
      },
    );
  }

  void refresh();

  function dispose() {
    disposed = true;
    void todosLifecycle.onTodosPageLeave();
    document.removeEventListener('plan-task-dialog-close', onDialogClose);
    closePlanTaskDialog();
    disposeFocusRefresh();
    if (typeof unlistenTurnCompleted === 'function') {
      void unlistenTurnCompleted();
      unlistenTurnCompleted = null;
    }
    container.removeEventListener('click', onClick);
    container.removeEventListener('input', onInput);
    container.removeEventListener('keydown', onKeydown);
    container.removeEventListener('focusout', onTitleBlur);
    container.removeEventListener('change', onChange);
    container.innerHTML = '';
  }

  // Match corpus-doc-list: unmount is callable and carries in-place route helpers.
  dispose.applyRoute = applyRoute;
  dispose.refresh = refresh;
  return { dispose, unmount: dispose, refresh, applyRoute };
}
