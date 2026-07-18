import { createApiClient, resolveReadDriver } from '../apiClient.js';
import { renderCommentMarkdown } from '../comment-markdown.js';
import { escHtml } from '../utils.js';
import { closePlanTaskDialog, openPlanTaskDialog } from './dialog.js';

const UNAVAILABLE_MSG = '列表暂时不可用，请稍后重试';
const REFRESH_WARNING_MSG = '已保存，列表刷新失败，请重试';
const MIGRATION_WARNING_MSG = '此待办的数据迁移未完成，部分信息可能不完整';
const ATTACHMENTS_EMPTY_MSG = '暂无附件';
const ATTACHMENT_PICK_CANCEL_MSG = '已取消选择文件';
const AI_ASSISTANT_TURN_COMPLETED = 'ai-assistant:turn-completed';
const COPY_MASTER_ID_LABEL = '复制任务 ID';
const COPY_FEEDBACK_LABEL = '✓ 已复制';
const COPY_FEEDBACK_MS = 1200;

const STATUS_LABELS = {
  incomplete: '进行中',
  complete: '已完成',
  abandoned: '已废弃',
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
  const data = await client.getJson('/api/plan-tasks');
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
  return invokePlanWrite('create_plan_task', args);
}

export async function deletePlanTask({ masterTaskId } = {}) {
  return invokePlanWrite('delete_plan_task', { masterTaskId });
}

export async function addPlanSub({ masterTaskId, title } = {}) {
  return invokePlanWrite('add_plan_sub', { masterTaskId, title });
}

export async function deletePlanSub({ masterTaskId, subTaskId } = {}) {
  return invokePlanWrite('delete_plan_sub', { masterTaskId, subTaskId });
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
  return invokePlanPlain('read_plan_md', { masterTaskId });
}

export async function updatePlanMd({ masterTaskId, planMd } = {}) {
  await invokePlanPlain('update_plan_md', { masterTaskId, planMd });
}

export async function completePlanSub({ masterTaskId, subTaskId } = {}) {
  return invokePlanWrite('complete_plan_sub', { masterTaskId, subTaskId });
}

export async function abandonPlanSub({ masterTaskId, subTaskId } = {}) {
  return invokePlanWrite('abandon_plan_sub', { masterTaskId, subTaskId });
}

export async function updatePlanSub({ masterTaskId, subTaskId, title } = {}) {
  return invokePlanWrite('update_plan_sub', { masterTaskId, subTaskId, title });
}

export async function listPlanAttachments({ masterTaskId } = {}) {
  const result = await invokePlanPlain('list_plan_attachments', { masterTaskId });
  if (Array.isArray(result)) return result;
  if (result && Array.isArray(result.attachments)) return result.attachments;
  return [];
}

export async function addPlanAttachment({ masterTaskId, fileName, content } = {}) {
  return invokePlanPlain('add_plan_attachment', { masterTaskId, fileName, content });
}

export async function readPlanAttachment({ masterTaskId, fileName } = {}) {
  return invokePlanPlain('read_plan_attachment', { masterTaskId, fileName });
}

export async function savePlanAttachment({ masterTaskId, fileName, content } = {}) {
  return invokePlanPlain('save_plan_attachment', { masterTaskId, fileName, content });
}

export async function deletePlanAttachment({ masterTaskId, fileName } = {}) {
  return invokePlanPlain('delete_plan_attachment', { masterTaskId, fileName });
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
    throw new Error('文件选择不可用');
  }
  const dialog = window.__TAURI__?.dialog;
  if (typeof dialog?.open !== 'function') {
    throw new Error('文件选择不可用');
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
    throw new Error('无法读取所选文件');
  }
  const res = await readAsset(convertFileSrc(path));
  if (!res.ok) {
    throw new Error('无法读取所选文件');
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
  if (minutes < 1) return '刚刚';
  if (minutes < 60) return `${minutes} 分钟前`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} 小时前`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days} 天前`;
  return new Date(time).toLocaleDateString('zh-CN');
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
      <button type="button" class="md-header-btn plan-task-refresh-retry"${disabledAttr} data-action="retry-refresh">重试刷新</button>
    </div>
  `;
}

function renderPageHeader(disabled) {
  const disabledAttr = disabled ? ' disabled' : '';
  return `
    <header class="plan-tasks-page-header">
      <h1 class="plan-tasks-page-title">Todos</h1>
      <button type="button" class="md-header-btn primary" data-action="create-master"${disabledAttr}>+ 新建待办</button>
    </header>
  `;
}

function renderMasterList(masters, selectedMasterId, disabled) {
  const disabledAttr = disabled ? ' disabled' : '';
  const items = sortMasters(masters)
    .map((master) => {
      const selected =
        master.master_task_id === selectedMasterId ? ' plan-task-master-item--selected' : '';
      const subCount = master.sub_tasks?.length ?? 0;
      const meta = `${subCount} 个子任务 · ${formatRelativeTime(master.created_at) || '未知时间'}`;
      return `
        <li>
          <button type="button" class="plan-task-master-item${selected}" data-master-id="${escHtml(master.master_task_id)}"${disabledAttr}>
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
      <p class="plan-task-empty-title">还没有待办</p>
      <p class="plan-task-empty-detail">创建第一个待办，开始管理子任务</p>
      <button type="button" class="md-header-btn primary" data-action="create-master"${disabledAttr}>新建待办</button>
    </div>
  `;
}

function renderMasterPane(masters, selectedMasterId, disabled) {
  if (!masters.length) {
    return renderMasterEmpty(disabled);
  }
  return renderMasterList(masters, selectedMasterId, disabled);
}

function renderLinkedArchives(linkedArchiveIds) {
  if (!linkedArchiveIds?.length) return '';
  return `<div class="plan-task-sub-archives">关联归档：${escHtml(linkedArchiveIds.join(', '))}</div>`;
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
      class="plan-task-sub-status-select plan-task-sub-status-select--${escHtml(sub.status)}"
      data-action="change-sub-status"
      data-sub-id="${escHtml(sub.sub_task_id)}"
      aria-label="子任务状态"${disabledAttr}
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
          aria-label="子任务标题"
          ${ui.disabled ? 'disabled' : ''}
        />
        <div class="plan-task-sub-header-actions">
          ${renderSubStatusSelect(subForRender, ui.disabled)}
          <div class="plan-task-sub-menu">
            <button type="button" class="plan-task-sub-menu-btn" data-action="toggle-sub-menu" aria-label="更多操作"${disabledAttr}>⋯</button>
            <div class="plan-task-sub-menu-panel" hidden>
              <button type="button" data-action="copy-sub-id" data-copy-text="${escHtml(copyText)}">复制 ID</button>
              <button type="button" data-action="delete-sub" data-sub-id="${escHtml(sub.sub_task_id)}" data-sub-title="${escHtml(title)}">删除</button>
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

export function renderSubDetail(master, selectedSubId) {
  const subs = master.sub_tasks ?? [];
  const ui = { disabled: false, subStatus: {}, subActionErrors: {} };
  const items = subs.map((sub) => renderSubRow(master, sub, selectedSubId, ui)).join('');
  return `
    <div class="plan-task-detail-body">
      <h2 class="plan-task-detail-title">${escHtml(master.title)}</h2>
      <div class="plan-task-sub-list">${items}</div>
    </div>
  `;
}

function renderDetailMeta(master) {
  const subCount = master.sub_tasks?.length ?? 0;
  const created = formatRelativeTime(master.created_at) || '未知时间';
  return `<p class="plan-task-detail-meta">${subCount} 个子任务 · 创建于 ${escHtml(created)}</p>`;
}

function renderDetailToolbar(masterTaskId, disabled) {
  const disabledAttr = disabled ? ' disabled' : '';
  return `
    <div class="plan-task-detail-toolbar">
      <button type="button" class="md-header-btn" data-action="open-ai-assistant" data-master-id="${escHtml(masterTaskId)}"${disabledAttr}>AI 助手</button>
      <button type="button" class="md-header-btn" data-action="add-sub" data-master-id="${escHtml(masterTaskId)}"${disabledAttr}>添加子任务</button>
      <button type="button" class="md-header-btn plan-task-btn-danger" data-action="delete-master"${disabledAttr}>删除待办</button>
    </div>
  `;
}

function renderSubEmpty(disabled) {
  const disabledAttr = disabled ? ' disabled' : '';
  return `
    <div class="plan-task-empty plan-task-empty--detail">
      <p class="plan-task-empty-title">还没有子任务</p>
      <button type="button" class="md-header-btn primary" data-action="add-sub"${disabledAttr}>添加子任务</button>
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
      <section class="plan-task-plan-md-section" aria-label="待办说明">
        <div class="plan-task-plan-md-header">
          <h3 class="plan-task-plan-md-title">待办说明</h3>
          <div class="plan-task-plan-md-header-actions">${copyBtn}</div>
        </div>
        <p class="plan-task-plan-md-loading">加载说明…</p>
      </section>
    `;
  }
  if (ui.planMdEditMode) {
    return `
      <section class="plan-task-plan-md-section" aria-label="待办说明">
        <div class="plan-task-plan-md-header">
          <h3 class="plan-task-plan-md-title">待办说明</h3>
          <div class="plan-task-plan-md-header-actions">${copyBtn}</div>
        </div>
        ${ui.planMdError ? `<p class="plan-task-plan-md-error" role="alert">${escHtml(ui.planMdError)}</p>` : ''}
        <textarea class="plan-task-plan-md-editor"${disabledAttr}>${escHtml(ui.planMdDraft ?? '')}</textarea>
        <div class="plan-task-plan-md-toolbar">
          <button type="button" class="md-header-btn primary" data-action="save-plan-md"${disabledAttr}>保存</button>
          <button type="button" class="md-header-btn" data-action="cancel-plan-md"${disabledAttr}>取消</button>
        </div>
      </section>
    `;
  }
  const planMd = master.plan_md ?? '';
  const previewHtml = planMd
    ? renderCommentMarkdown(planMd)
    : '<p class="plan-task-plan-md-empty">暂无说明</p>';
  return `
    <section class="plan-task-plan-md-section" aria-label="待办说明">
      <div class="plan-task-plan-md-header">
        <h3 class="plan-task-plan-md-title">待办说明</h3>
        <div class="plan-task-plan-md-header-actions">
          ${copyBtn}
          <button type="button" class="md-header-btn" data-action="edit-plan-md"${disabledAttr}>编辑</button>
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
    ? `<span class="plan-task-attachments-count" aria-label="${items.length} 个附件">${items.length}</span>`
    : '';
  const emptyHtml = items.length
    ? ''
    : `<span class="plan-task-attachments-empty">${escHtml(ATTACHMENTS_EMPTY_MSG)}</span>`;
  const listHtml = items.length
    ? `<ul class="plan-task-attachment-list" role="list">${items
        .map((entry) => {
          const added = formatRelativeTime(entry.added_at);
          const meta = added ? `添加于 ${added}` : 'Markdown 附件';
          return `
        <li
          class="plan-task-attachment-item"
          data-action="open-attachment"
          data-file-name="${escHtml(entry.file_name)}"
          role="button"
          tabindex="0"
          aria-label="打开附件 ${escHtml(entry.file_name)}"
        >
          <span class="plan-task-attachment-icon" aria-hidden="true">MD</span>
          <span class="plan-task-attachment-main">
            <span class="plan-task-attachment-name">${escHtml(entry.file_name)}</span>
            <span class="plan-task-attachment-meta">${escHtml(meta)}</span>
          </span>
          <span class="plan-task-attachment-open-hint" aria-hidden="true">打开</span>
          <button
            type="button"
            class="plan-task-attachment-delete"
            data-action="delete-attachment"
            data-file-name="${escHtml(entry.file_name)}"
            aria-label="删除附件 ${escHtml(entry.file_name)}"
            title="删除"
            ${disabledAttr}
          >删除</button>
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
        aria-label="删除附件确认"
      >
        <div class="plan-task-attachment-delete-confirm-backdrop" data-action="cancel-delete-attachment"></div>
        <div class="plan-task-attachment-delete-confirm-panel">
          <h4 class="plan-task-attachment-delete-confirm-title">删除附件</h4>
          <p class="plan-task-attachment-delete-confirm-body">确认删除「${escHtml(confirmFile)}」？删除后列表与文件将一并移除。</p>
          <div class="plan-task-attachment-delete-confirm-actions">
            <button
              type="button"
              class="md-header-btn"
              data-action="cancel-delete-attachment"
              ${disabledAttr}
            >取消</button>
            <button
              type="button"
              class="md-header-btn plan-task-btn-danger"
              data-action="confirm-delete-attachment"
              data-file-name="${escHtml(confirmFile)}"
              ${disabledAttr}
            >确认删除</button>
          </div>
        </div>
      </div>`
    : '';
  return `
    <section class="plan-task-attachments-section" aria-label="附件">
      <div class="plan-task-attachments-header">
        <div class="plan-task-attachments-heading">
          <h3 class="plan-task-attachments-title">附件</h3>
          ${countHtml}
        </div>
        <div class="plan-task-attachments-header-actions">
          <button type="button" class="md-header-btn" data-action="pick-attachment-md"${disabledAttr}>本地选 .md</button>
          ${emptyHtml}
        </div>
      </div>
      ${errHtml}
      ${listHtml}
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
  const modeLabel = editor.loading ? '加载中' : editor.editMode ? '编辑' : '预览';
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
        <span>正在加载附件…</span>
      </div>`;
  } else if (editor.editMode) {
    bodyHtml = `
      ${errHtml}
      <label class="plan-task-attachment-edit-label" for="plan-task-attachment-edit-area">Markdown 原文</label>
      <textarea
        id="plan-task-attachment-edit-area"
        class="plan-task-attachment-edit-area"
        spellcheck="false"
        ${disabledAttr}
      >${escHtml(editor.content ?? '')}</textarea>
    `;
    footerHtml = `
      <footer class="plan-task-attachment-editor-footer">
        <span class="plan-task-attachment-editor-footer-hint">保存将覆盖该附件内容</span>
        <div class="plan-task-attachment-toolbar">
          <button type="button" class="md-header-btn" data-action="cancel-attachment-edit"${disabledAttr}>取消</button>
          <button type="button" class="md-header-btn primary" data-action="save-attachment"${disabledAttr}>保存</button>
        </div>
      </footer>`;
  } else {
    const previewHtml = editor.content
      ? renderCommentMarkdown(editor.content)
      : '<p class="plan-task-attachment-empty">暂无内容</p>';
    bodyHtml = `
      ${errHtml}
      <article class="plan-task-attachment-doc">
        <div class="plan-task-attachment-preview">${previewHtml}</div>
      </article>
    `;
  }
  const editBtn =
    !editor.loading && !editor.editMode
      ? `<button type="button" class="md-header-btn primary" data-action="edit-attachment"${disabledAttr}>编辑</button>`
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
            <button type="button" class="md-header-btn" data-action="close-attachment-editor"${disabledAttr}>关闭</button>
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
          <h2 class="plan-task-detail-title">${escHtml(master.title)}</h2>
          ${renderDetailMeta(master)}
        </div>
      </div>
      ${master.migration_error ? renderMigrationWarning() : ''}
      ${renderPlanMdSection(master, ui)}
      ${renderAttachmentsSection(ui)}
      ${renderRefreshWarning(ui.refreshWarning, ui.disabled)}
      ${renderDetailToolbar(master.master_task_id, ui.disabled)}
      <div class="plan-task-sub-list">${items}</div>
    </div>
  `;
}

function renderDetailEmpty() {
  return `
    <div class="plan-task-split-detail-empty plan-task-empty">
      <p class="plan-task-empty-title">选择左侧待办</p>
      <p class="plan-task-empty-detail">或点击右上角新建待办</p>
    </div>
  `;
}

function renderDeadLink() {
  return `
    <div class="plan-task-split-dead-link plan-task-split-state">
      <p class="plan-task-split-state-title">未找到对应任务</p>
      <p class="plan-task-split-state-detail">链接可能已失效，请从列表重新选择</p>
    </div>
  `;
}

function renderErrorEmpty(message = UNAVAILABLE_MSG) {
  return `
    <div class="plan-task-split-error plan-task-split-state plan-task-split-state--error">
      <p class="plan-task-split-state-title">暂时无法加载</p>
      <p class="plan-task-split-state-detail">${escHtml(message)}</p>
    </div>
  `;
}

function renderPageShell({ masterHtml, detailHtml, disabled = false }) {
  return `
    <div class="plan-tasks-page">
      ${renderPageHeader(disabled)}
      <div class="plan-task-split">
        <aside class="plan-task-split-master" aria-label="Todos列表">${masterHtml}</aside>
        <section class="plan-task-split-detail" aria-label="任务详情">${detailHtml}</section>
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
  const optimisticSubStatus = {};
  const subActionErrors = {};
  const subTitleErrors = {};
  const subTitleDrafts = {};

  container.innerHTML = '<div class="plan-task-split-loading">加载中…</div>';

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
      subStatus: optimisticSubStatus,
      subActionErrors,
      subTitleErrors,
      subTitleDrafts,
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

  function resolveSelection() {
    deadLink = false;
    if (!selectedMasterId) {
      selectedSubId = '';
      return;
    }
    const master = findMaster(selectedMasterId);
    if (!master) {
      deadLink = true;
      selectedSubId = '';
      return;
    }
    const subs = master.sub_tasks ?? [];
    if (validateInitialSubLink && initialSubId && selectedSubId === initialSubId) {
      validateInitialSubLink = false;
      const sub = subs.find((item) => item.sub_task_id === selectedSubId);
      if (!sub) {
        deadLink = true;
        selectedSubId = '';
      }
      return;
    }
    if (!selectedSubId || !subs.some((item) => item.sub_task_id === selectedSubId)) {
      const fallback = pickDefaultSub(master);
      selectedSubId = fallback?.sub_task_id ?? '';
    }
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
    const ui = getUi();
    const masterHtml = renderMasterPane(masters, selectedMasterId, ui.disabled);
    // Keep the same editor element node across paints so in-flight UI refs stay valid.
    const existingEditor = container.querySelector('.plan-task-attachment-editor');
    if (existingEditor) existingEditor.remove();
    container.innerHTML = renderPageShell({
      masterHtml,
      detailHtml: renderDetailPane(),
      disabled: ui.disabled,
    });
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
        error: err?.message || '加载附件失败',
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
        error: err?.message || '保存失败',
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
      if (disposed) return;
      paint();
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
      refreshWarning = '';
      container.innerHTML = renderPageShell({
        masterHtml: '<div class="plan-task-split-state"></div>',
        detailHtml: renderErrorEmpty(),
      });
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
        ? `选择文件失败：${err.message}`
        : '选择文件失败';
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
      attachmentsError = err?.message || '添加附件失败';
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
      attachmentsError = err?.message || '删除附件失败';
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
      planMdError = err?.message || '加载说明失败';
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
      planMdError = err?.message || '保存失败';
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
      subActionErrors[subTaskId] = err?.message || '操作失败';
      paint();
    }
  }

  async function runSubTitleSave(subTaskId, title) {
    const master = findMaster(selectedMasterId);
    const sub = master?.sub_tasks?.find((item) => item.sub_task_id === subTaskId);
    if (!master || !sub) return;
    const trimmed = title.trim();
    if (!trimmed) {
      subTitleErrors[subTaskId] = '标题不能为空';
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
      subTitleErrors[subTaskId] = err?.message || '保存失败';
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
        await runSubStatusAction(subTaskId, 'complete', completePlanSub);
      } else {
        await runSubStatusAction(subTaskId, 'abandoned', abandonPlanSub);
      }
    } catch {
      if (selectEl instanceof HTMLSelectElement) {
        selectEl.value = priorStatus;
      }
    }
  }

  async function openPlanAiAssistant() {
    if (!selectedMasterId || controlsDisabled(busy)) return;
    const invoke = getTauriInvoke();
    if (!invoke) return;
    try {
      await invoke('open_ai_assistant', { masterTaskId: selectedMasterId });
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

    if (action === 'create-master') {
      event.preventDefault();
      if (controlsDisabled(busy)) return;
      openCreateDialog(actionEl instanceof HTMLElement ? actionEl : null);
      return;
    }

    if (action === 'open-ai-assistant') {
      event.preventDefault();
      if (controlsDisabled(busy) || !selectedMasterId) return;
      void openPlanAiAssistant();
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
      closeSubMenus();
      resetPlanMdEdit();
      closeAttachmentEditor();
      clearSubActionState();
      clearSubTitleState();
      selectedMasterId = masterBtn.dataset.masterId;
      const master = findMaster(selectedMasterId);
      const fallback = master ? pickDefaultSub(master) : null;
      selectedSubId = fallback?.sub_task_id ?? '';
      deadLink = false;
      attachments = [];
      attachmentsError = '';
      attachmentDeleteConfirm = '';
      void (async () => {
        await loadAttachmentsForSelected();
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
    const input = event.target.closest('[data-action="edit-sub-title"]');
    if (!(input instanceof HTMLInputElement) || controlsDisabled(busy) || !selectedMasterId) return;
    subTitleDrafts[input.dataset.subId ?? ''] = input.value;
  };

  const onKeydown = (event) => {
    if (event.key === 'Escape') {
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

    const input = event.target.closest('[data-action="edit-sub-title"]');
    if (!(input instanceof HTMLInputElement)) return;
    if (event.key === 'Enter') {
      event.preventDefault();
      input.blur();
    }
  };

  const onSubTitleBlur = (event) => {
    const input = event.target.closest('[data-action="edit-sub-title"]');
    if (!(input instanceof HTMLInputElement) || controlsDisabled(busy) || !selectedMasterId) return;
    const subTaskId = input.dataset.subId;
    if (!subTaskId) return;
    void runSubTitleSave(subTaskId, input.value);
  };

  const onChange = (event) => {
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
  container.addEventListener('focusout', onSubTitleBlur);
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
    container.removeEventListener('focusout', onSubTitleBlur);
    container.removeEventListener('change', onChange);
    container.innerHTML = '';
  }

  return { dispose, unmount: dispose, refresh };
}
