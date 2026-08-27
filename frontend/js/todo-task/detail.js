import { escHtml } from '../utils.js';
import {
  DEFAULT_PLAN_CATEGORY_ID,
  abandonPlanSub,
  completePlan,
  setPlanMasterStatus,
  updatePlanMasterTitle,
  updatePlanSub,
} from './host.js';
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

export function createDetailOwner(ctx) {
  const optimisticSubStatus = {};
  const subActionErrors = {};
  const subTitleErrors = {};
  const subTitleDrafts = {};
  const expandedSubContent = {};
  const subContentDrafts = {};
  let masterTitleDraft = '';
  let masterTitleError = '';

  function clearSubTitleState() {
    for (const key of Object.keys(subTitleDrafts)) delete subTitleDrafts[key];
    for (const key of Object.keys(subTitleErrors)) delete subTitleErrors[key];
  }

  function clearSubActionState() {
    for (const key of Object.keys(optimisticSubStatus)) delete optimisticSubStatus[key];
    for (const key of Object.keys(subActionErrors)) delete subActionErrors[key];
  }

  function resetForSelectionChange() {
    clearSubActionState();
    clearSubTitleState();
    masterTitleDraft = '';
    masterTitleError = '';
  }

  async function runSubStatusAction(subTaskId, targetStatus, actionFn) {
    const master = ctx.findSelectedMaster();
    const sub = master?.sub_tasks?.find((item) => item.sub_task_id === subTaskId);
    if (!master || !sub || sub.status !== 'incomplete') return;
    delete subActionErrors[subTaskId];
    optimisticSubStatus[subTaskId] = targetStatus;
    ctx.setBusy(true);
    ctx.paint();
    try {
      await actionFn({ masterTaskId: ctx.getSelectedMasterId(), subTaskId });
      delete optimisticSubStatus[subTaskId];
      ctx.setBusy(false);
      await ctx.reloadList({ afterWrite: true });
    } catch (err) {
      delete optimisticSubStatus[subTaskId];
      ctx.setBusy(false);
      subActionErrors[subTaskId] = err?.message || 'Operation failed';
      ctx.paint();
    }
  }

  async function runSubTitleSave(subTaskId, title) {
    const master = ctx.findSelectedMaster();
    const sub = master?.sub_tasks?.find((item) => item.sub_task_id === subTaskId);
    if (!master || !sub) return;
    const trimmed = title.trim();
    if (!trimmed) {
      subTitleErrors[subTaskId] = 'Title cannot be empty';
      ctx.paint();
      return;
    }
    if (trimmed === (sub.title || sub.sub_task_id)) {
      delete subTitleDrafts[subTaskId];
      delete subTitleErrors[subTaskId];
      return;
    }
    delete subTitleErrors[subTaskId];
    subTitleDrafts[subTaskId] = trimmed;
    ctx.setBusy(true);
    ctx.paint();
    try {
      await updatePlanSub({
        masterTaskId: ctx.getSelectedMasterId(),
        subTaskId,
        title: trimmed,
      });
      delete subTitleDrafts[subTaskId];
      ctx.setBusy(false);
      await ctx.reloadList({ afterWrite: true });
    } catch (err) {
      delete subTitleDrafts[subTaskId];
      ctx.setBusy(false);
      subTitleErrors[subTaskId] = err?.message || 'Save failed';
      ctx.paint();
    }
  }

  async function runSubContentSave(subTaskId, content) {
    const master = ctx.findSelectedMaster();
    const sub = master?.sub_tasks?.find((item) => item.sub_task_id === subTaskId);
    if (!master || !sub) return;
    const next = typeof content === 'string' ? content : '';
    const prev = typeof sub.content === 'string' ? sub.content : '';
    if (next === prev) {
      delete subContentDrafts[subTaskId];
      return;
    }
    subContentDrafts[subTaskId] = next;
    delete subActionErrors[subTaskId];
    ctx.setBusy(true);
    ctx.paint();
    try {
      await updatePlanSub({
        masterTaskId: ctx.getSelectedMasterId(),
        subTaskId,
        title: sub.title || sub.sub_task_id,
        content: next,
      });
      delete subContentDrafts[subTaskId];
      ctx.setBusy(false);
      await ctx.reloadList({ afterWrite: true });
    } catch (err) {
      ctx.setBusy(false);
      subActionErrors[subTaskId] = err?.message || 'Save failed';
      ctx.paint();
    }
  }

  async function runMasterTitleSave(title) {
    const master = ctx.findSelectedMaster();
    if (!master) return;
    const trimmed = title.trim();
    if (!trimmed) {
      masterTitleError = 'Title cannot be empty';
      ctx.paint();
      return;
    }
    if (trimmed === master.title) {
      masterTitleDraft = '';
      masterTitleError = '';
      return;
    }
    masterTitleError = '';
    masterTitleDraft = trimmed;
    ctx.setBusy(true);
    ctx.paint();
    try {
      await updatePlanMasterTitle({
        masterTaskId: ctx.getSelectedMasterId(),
        title: trimmed,
      });
      masterTitleDraft = '';
      ctx.setBusy(false);
      await ctx.reloadList({ afterWrite: true });
    } catch (err) {
      ctx.setBusy(false);
      masterTitleError = err?.message || 'Save failed';
      ctx.paint();
    }
  }

  async function runSubStatusChange(subTaskId, targetStatus, selectEl) {
    const master = ctx.findSelectedMaster();
    const sub = master?.sub_tasks?.find((item) => item.sub_task_id === subTaskId);
    const priorStatus = sub?.status ?? 'incomplete';
    if (!master || !sub || sub.status !== 'incomplete') {
      if (selectEl instanceof HTMLSelectElement) selectEl.value = priorStatus;
      return;
    }
    if (targetStatus === priorStatus) return;
    if (targetStatus !== 'complete' && targetStatus !== 'abandoned') {
      if (selectEl instanceof HTMLSelectElement) selectEl.value = priorStatus;
      return;
    }
    try {
      if (targetStatus === 'complete') {
        await runSubStatusAction(subTaskId, 'complete', completePlan);
      } else {
        await runSubStatusAction(subTaskId, 'abandoned', abandonPlanSub);
      }
    } catch {
      if (selectEl instanceof HTMLSelectElement) selectEl.value = priorStatus;
    }
  }

  async function runMasterStatusChange(targetStatus, selectEl) {
    const master = ctx.findSelectedMaster();
    const priorStatus = masterStatusClass(master?.status);
    if (!master) {
      if (selectEl instanceof HTMLSelectElement) selectEl.value = priorStatus;
      return;
    }
    if (
      targetStatus !== 'incomplete' &&
      targetStatus !== 'complete' &&
      targetStatus !== 'abandoned'
    ) {
      if (selectEl instanceof HTMLSelectElement) selectEl.value = priorStatus;
      return;
    }
    if (targetStatus === priorStatus) return;
    ctx.setBusy(true);
    ctx.paint();
    try {
      await setPlanMasterStatus({
        masterTaskId: ctx.getSelectedMasterId(),
        status: targetStatus,
      });
      ctx.setBusy(false);
      await ctx.reloadList({ afterWrite: true });
    } catch {
      ctx.setBusy(false);
      if (selectEl instanceof HTMLSelectElement) selectEl.value = priorStatus;
      ctx.paint();
    }
  }

  return {
    uiSlice() {
      return {
        subStatus: optimisticSubStatus,
        subActionErrors,
        subTitleErrors,
        subTitleDrafts,
        expandedSubContent,
        subContentDrafts,
        masterTitleDraft: masterTitleDraft || undefined,
        masterTitleError,
      };
    },
    resetForSelectionChange,
    setMasterTitleDraft(value) {
      masterTitleDraft = value;
    },
    setSubTitleDraft(subTaskId, value) {
      subTitleDrafts[subTaskId] = value;
    },
    setSubContentDraft(subTaskId, value) {
      subContentDrafts[subTaskId] = value;
    },
    toggleSubContent(subTaskId) {
      if (expandedSubContent[subTaskId]) delete expandedSubContent[subTaskId];
      else expandedSubContent[subTaskId] = true;
    },
    handleClick(event, action, actionEl) {
      if (action === 'toggle-sub-content') {
        event.stopPropagation();
        if (ctx.isBusy()) return true;
        const subTaskId = actionEl?.dataset.subId;
        if (!subTaskId) return true;
        this.toggleSubContent(subTaskId);
        ctx.paint();
        return true;
      }
      return false;
    },
    handleInput(event) {
      const masterTitleInput = event.target.closest('[data-action="edit-master-title"]');
      if (masterTitleInput instanceof HTMLInputElement) {
        if (ctx.isBusy() || !ctx.getSelectedMasterId()) return true;
        masterTitleDraft = masterTitleInput.value;
        return true;
      }
      const contentInput = event.target.closest('[data-action="edit-sub-content"]');
      if (contentInput instanceof HTMLTextAreaElement) {
        if (ctx.isBusy() || !ctx.getSelectedMasterId()) return true;
        subContentDrafts[contentInput.dataset.subId ?? ''] = contentInput.value;
        return true;
      }
      const input = event.target.closest('[data-action="edit-sub-title"]');
      if (!(input instanceof HTMLInputElement) || ctx.isBusy() || !ctx.getSelectedMasterId()) {
        return Boolean(input);
      }
      subTitleDrafts[input.dataset.subId ?? ''] = input.value;
      return true;
    },
    handleBlur(event) {
      const masterTitleInput = event.target.closest('[data-action="edit-master-title"]');
      if (masterTitleInput instanceof HTMLInputElement) {
        if (ctx.isBusy() || !ctx.getSelectedMasterId()) return true;
        void runMasterTitleSave(masterTitleInput.value);
        return true;
      }
      const contentInput = event.target.closest('[data-action="edit-sub-content"]');
      if (contentInput instanceof HTMLTextAreaElement) {
        if (ctx.isBusy() || !ctx.getSelectedMasterId()) return true;
        const subTaskId = contentInput.dataset.subId;
        if (!subTaskId) return true;
        void runSubContentSave(subTaskId, contentInput.value);
        return true;
      }
      const input = event.target.closest('[data-action="edit-sub-title"]');
      if (!(input instanceof HTMLInputElement) || ctx.isBusy() || !ctx.getSelectedMasterId()) {
        return Boolean(input instanceof HTMLInputElement);
      }
      const subTaskId = input.dataset.subId;
      if (!subTaskId) return true;
      void runSubTitleSave(subTaskId, input.value);
      return true;
    },
    handleChange(event) {
      const masterSelect = event.target.closest('[data-action="change-master-status"]');
      if (masterSelect instanceof HTMLSelectElement) {
        if (ctx.isBusy() || !ctx.getSelectedMasterId()) {
          const master = ctx.findSelectedMaster();
          masterSelect.value = masterStatusClass(master?.status);
          return true;
        }
        void runMasterStatusChange(masterSelect.value, masterSelect);
        return true;
      }
      const select = event.target.closest('[data-action="change-sub-status"]');
      if (!(select instanceof HTMLSelectElement)) return false;
      if (ctx.isBusy() || !ctx.getSelectedMasterId()) {
        select.value = select.dataset.currentStatus ?? select.value;
        return true;
      }
      const subTaskId = select.dataset.subId;
      if (!subTaskId) return true;
      void runSubStatusChange(subTaskId, select.value, select);
      return true;
    },
    handleTitleEnter(event) {
      const titleInput = event.target.closest(
        '[data-action="edit-master-title"], [data-action="edit-sub-title"]',
      );
      if (!(titleInput instanceof HTMLInputElement)) return false;
      if (event.key === 'Enter') {
        event.preventDefault();
        titleInput.blur();
        return true;
      }
      return false;
    },
  };
}
