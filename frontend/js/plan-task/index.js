import { createApiClient, resolveReadDriver } from '../apiClient.js';
import { escHtml } from '../utils.js';
import { closePlanTaskDialog, isPlanTaskDialogOpen, openPlanTaskDialog } from './dialog.js';

const UNAVAILABLE_MSG = '列表暂时不可用，请稍后重试';
const REFRESH_WARNING_MSG = '已保存，列表刷新失败，请重试';

const STATUS_LABELS = {
  incomplete: '进行中',
  complete: '已完成',
};

function getTauriInvoke() {
  if (typeof window === 'undefined') return null;
  const invoke =
    window.__TAURI__?.core?.invoke || window.__TAURI_INTERNALS__?.invoke;
  return typeof invoke === 'function' ? invoke : null;
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
  return busy || isPlanTaskDialogOpen();
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
      <h1 class="plan-tasks-page-title">计划任务</h1>
      <button type="button" class="md-header-btn primary" data-action="create-master"${disabledAttr}>+ 新建计划</button>
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
      <p class="plan-task-empty-title">还没有计划</p>
      <p class="plan-task-empty-detail">创建第一个计划，开始管理子任务</p>
      <button type="button" class="md-header-btn primary" data-action="create-master"${disabledAttr}>新建计划</button>
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

function renderSubRow(master, sub, selectedSubId, disabled) {
  const selected = sub.sub_task_id === selectedSubId ? ' plan-task-sub--selected' : '';
  const title = sub.title || sub.sub_task_id;
  const statusLabel = formatPlanTaskStatus(sub.status);
  const disabledAttr = disabled ? ' disabled' : '';
  const copyText = copySubIdPair(master.master_task_id, sub.sub_task_id);
  return `
    <article data-sub-id="${escHtml(sub.sub_task_id)}" class="plan-task-sub${selected}">
      <header class="plan-task-sub-header">
        <h3 class="plan-task-sub-title">${escHtml(title)}</h3>
        <div class="plan-task-sub-header-actions">
          <span class="plan-task-sub-status plan-task-sub-status--${escHtml(sub.status)}">${escHtml(statusLabel)}</span>
          <div class="plan-task-sub-menu">
            <button type="button" class="plan-task-sub-menu-btn" data-action="toggle-sub-menu" aria-label="更多操作"${disabledAttr}>⋯</button>
            <div class="plan-task-sub-menu-panel" hidden>
              <button type="button" data-action="copy-sub-id" data-copy-text="${escHtml(copyText)}">复制 ID</button>
              <button type="button" data-action="delete-sub" data-sub-id="${escHtml(sub.sub_task_id)}" data-sub-title="${escHtml(title)}">删除</button>
            </div>
          </div>
        </div>
      </header>
      ${renderLinkedArchives(sub.linked_archive_ids)}
    </article>
  `;
}

export function renderSubDetail(master, selectedSubId) {
  const subs = master.sub_tasks ?? [];
  const items = subs.map((sub) => renderSubRow(master, sub, selectedSubId, false)).join('');
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
      <button type="button" class="md-header-btn" data-action="add-sub" data-master-id="${escHtml(masterTaskId)}"${disabledAttr}>添加子任务</button>
      <button type="button" class="md-header-btn plan-task-btn-danger" data-action="delete-master"${disabledAttr}>删除计划</button>
    </div>
  `;
}

function renderSubEmpty(disabled) {
  const disabledAttr = disabled ? ' disabled' : '';
  return `
    <div class="plan-task-empty plan-task-empty--detail">
      <p class="plan-task-empty-title">还没有子任务</p>
      <p class="plan-task-empty-detail">添加第一个子任务开始执行</p>
      <button type="button" class="md-header-btn primary" data-action="add-sub"${disabledAttr}>添加子任务</button>
    </div>
  `;
}

function renderSubDetailPane(master, selectedSubId, ui) {
  const subs = master.sub_tasks ?? [];
  const items = subs.length
    ? subs.map((sub) => renderSubRow(master, sub, selectedSubId, ui.disabled)).join('')
    : renderSubEmpty(ui.disabled);
  return `
    <div class="plan-task-detail-body">
      <div class="plan-task-detail-header">
        <div>
          <h2 class="plan-task-detail-title">${escHtml(master.title)}</h2>
          ${renderDetailMeta(master)}
        </div>
      </div>
      ${renderRefreshWarning(ui.refreshWarning, ui.disabled)}
      ${renderDetailToolbar(master.master_task_id, ui.disabled)}
      <div class="plan-task-sub-list">${items}</div>
    </div>
  `;
}

function renderDetailEmpty() {
  return `
    <div class="plan-task-split-detail-empty plan-task-empty">
      <p class="plan-task-empty-title">选择左侧计划</p>
      <p class="plan-task-empty-detail">或点击右上角新建计划</p>
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
        <aside class="plan-task-split-master" aria-label="计划任务列表">${masterHtml}</aside>
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
  let refreshPromise = null;
  let busy = false;
  let refreshWarning = '';

  container.innerHTML = '<div class="plan-task-split-loading">加载中…</div>';

  function findMaster(id) {
    return masters.find((master) => master.master_task_id === id) ?? null;
  }

  function getUi() {
    return { disabled: controlsDisabled(busy), refreshWarning };
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
    if (initialSubId && selectedSubId === initialSubId) {
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

  function paint() {
    if (!masters.length && container.querySelector('.plan-task-split-error')) {
      return;
    }
    const ui = getUi();
    const masterHtml = renderMasterPane(masters, selectedMasterId, ui.disabled);
    container.innerHTML = renderPageShell({
      masterHtml,
      detailHtml: renderDetailPane(),
      disabled: ui.disabled,
    });
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
      refreshWarning = '';
      container.innerHTML = renderPageShell({
        masterHtml: '<div class="plan-task-split-state"></div>',
        detailHtml: renderErrorEmpty(),
      });
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
    try {
      await actionFn();
      busy = false;
      await reloadList({ afterWrite: true });
    } catch (err) {
      busy = false;
      throw err;
    }
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
        await runWriteAction(async () => {
          await deletePlanSub({ masterTaskId, subTaskId });
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

    if (action === 'create-master') {
      event.preventDefault();
      if (controlsDisabled(busy)) return;
      openCreateDialog(actionEl instanceof HTMLElement ? actionEl : null);
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

    if (action === 'copy-sub-id') {
      event.preventDefault();
      event.stopPropagation();
      const text = actionEl?.dataset.copyText ?? '';
      if (text && navigator.clipboard?.writeText) {
        void navigator.clipboard.writeText(text);
      }
      closeSubMenus();
      return;
    }

    const masterBtn = event.target.closest('.plan-task-master-item');
    if (masterBtn?.dataset.masterId) {
      if (controlsDisabled(busy)) return;
      closeSubMenus();
      selectedMasterId = masterBtn.dataset.masterId;
      const master = findMaster(selectedMasterId);
      const fallback = master ? pickDefaultSub(master) : null;
      selectedSubId = fallback?.sub_task_id ?? '';
      deadLink = false;
      paint();
      if (typeof navigate === 'function' && selectedMasterId && selectedSubId) {
        navigate(buildDeepLink(selectedMasterId, selectedSubId));
      }
      return;
    }

    const subEl = event.target.closest('.plan-task-sub');
    if (subEl?.dataset.subId && selectedMasterId) {
      if (controlsDisabled(busy)) return;
      if (event.target.closest('.plan-task-sub-menu')) return;
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

  container.addEventListener('click', onClick);
  const disposeFocusRefresh = bindFocusRefresh(refresh);
  void refresh();

  function dispose() {
    disposed = true;
    closePlanTaskDialog();
    disposeFocusRefresh();
    container.removeEventListener('click', onClick);
    container.innerHTML = '';
  }

  return { dispose, unmount: dispose, refresh };
}
