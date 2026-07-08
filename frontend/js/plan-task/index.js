import { createApiClient, resolveReadDriver } from '../apiClient.js';
import { escHtml } from '../utils.js';

const UNAVAILABLE_MSG = '列表暂时不可用，请稍后重试';
const REFRESH_WARNING_MSG = '已保存，列表刷新失败，请重试';

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

function writeControlsDisabled(writeState) {
  return writeState === 'writing' || writeState === 'refresh';
}

function parseSubTitles(raw) {
  const trimmed = raw?.trim();
  if (!trimmed) return undefined;
  const titles = trimmed
    .split(',')
    .map((part) => part.trim())
    .filter(Boolean);
  return titles.length ? titles : undefined;
}

function renderWriteError(writeError) {
  if (!writeError) return '';
  return `
    <div class="plan-task-write-error plan-task-split-state plan-task-split-state--error" role="alert">
      <p class="plan-task-split-state-detail">${escHtml(writeError)}</p>
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

function renderMasterToolbar({ writeState, inputKind }) {
  const disabled = writeControlsDisabled(writeState);
  const disabledAttr = disabled ? ' disabled' : '';
  if (writeState === 'input' && inputKind === 'create-master') {
    return `
      <div class="plan-task-master-toolbar plan-task-write-input" data-write-zone="create-master">
        <label class="plan-task-write-label">
          <span>计划标题</span>
          <input type="text" class="plan-task-write-field" data-input="create-title" placeholder="必填" />
        </label>
        <label class="plan-task-write-label">
          <span>子任务标题（可选，逗号分隔）</span>
          <input type="text" class="plan-task-write-field" data-input="create-subtitles" placeholder="例如：调研, 实现" />
        </label>
        <div class="plan-task-write-actions">
          <button type="button" class="md-header-btn primary" data-action="submit-create-master">创建</button>
          <button type="button" class="md-header-btn" data-action="cancel-input">取消</button>
        </div>
      </div>
    `;
  }
  return `
    <div class="plan-task-master-toolbar">
      <button type="button" class="md-header-btn primary plan-task-write-btn" data-action="create-master"${disabledAttr}>新建计划</button>
    </div>
  `;
}

function renderMasterList(masters, selectedMasterId) {
  const items = sortMasters(masters)
    .map((master) => {
      const selected =
        master.master_task_id === selectedMasterId ? ' plan-task-master-item--selected' : '';
      return `
        <li>
          <button type="button" class="plan-task-master-item${selected}" data-master-id="${escHtml(master.master_task_id)}">
            <span class="plan-task-master-title">${escHtml(master.title)}</span>
          </button>
        </li>
      `;
    })
    .join('');
  return `<ul class="plan-task-master-list" role="list">${items}</ul>`;
}

function renderMasterPane(masters, selectedMasterId, writeUi) {
  const toolbar = renderMasterToolbar(writeUi);
  const list = masters.length
    ? renderMasterList(masters, selectedMasterId)
    : '<div class="plan-task-split-state"><p class="plan-task-split-state-title">暂无计划任务</p></div>';
  const masterError =
    writeUi.writeError && writeUi.inputKind === 'create-master' ? writeUi.writeError : '';
  return `${toolbar}${renderWriteError(masterError)}${list}`;
}

function renderLinkedArchives(linkedArchiveIds) {
  if (!linkedArchiveIds?.length) return '';
  return `<div class="plan-task-sub-archives">关联归档：${escHtml(linkedArchiveIds.join(', '))}</div>`;
}

function renderSubRow(master, sub, selectedSubId, writeUi) {
  const selected = sub.sub_task_id === selectedSubId ? ' plan-task-sub--selected' : '';
  const copyText = copySubIdPair(master.master_task_id, sub.sub_task_id);
  const title = sub.title || sub.sub_task_id;
  const disabled = writeControlsDisabled(writeUi.writeState);
  const disabledAttr = disabled ? ' disabled' : '';
  return `
    <article data-sub-id="${escHtml(sub.sub_task_id)}" class="plan-task-sub${selected}">
      <header class="plan-task-sub-header">
        <h3 class="plan-task-sub-title">${escHtml(title)}</h3>
        <div class="plan-task-sub-header-actions">
          <span class="plan-task-sub-status plan-task-sub-status--${escHtml(sub.status)}">${escHtml(sub.status)}</span>
          <button type="button" class="plan-task-sub-delete" data-action="delete-sub" data-sub-id="${escHtml(sub.sub_task_id)}" aria-label="删除子任务"${disabledAttr}>×</button>
        </div>
      </header>
      <div class="plan-task-sub-copy">${escHtml(copyText)}</div>
      ${renderLinkedArchives(sub.linked_archive_ids)}
    </article>
  `;
}

export function renderSubDetail(master, selectedSubId) {
  const subs = master.sub_tasks ?? [];
  const items = subs.map((sub) => renderSubRow(master, sub, selectedSubId, { writeState: 'idle' })).join('');
  return `
    <div class="plan-task-detail-body">
      <h2 class="plan-task-detail-title">${escHtml(master.title)}</h2>
      <div class="plan-task-sub-list">${items}</div>
    </div>
  `;
}

function renderDetailHeader(master, writeUi) {
  const disabled = writeControlsDisabled(writeUi.writeState);
  const disabledAttr = disabled ? ' disabled' : '';
  if (writeUi.pendingDeleteMaster) {
    return `
      <div class="plan-task-detail-header plan-task-delete-confirm" data-write-zone="delete-master">
        <h2 class="plan-task-detail-title">${escHtml(master.title)}</h2>
        <div class="plan-task-write-actions">
          <span class="plan-task-delete-confirm-text">确认删除此计划？</span>
          <button type="button" class="md-header-btn primary" data-action="confirm-delete-master">确认删除</button>
          <button type="button" class="md-header-btn" data-action="cancel-delete-master">取消</button>
        </div>
      </div>
    `;
  }
  return `
    <div class="plan-task-detail-header">
      <h2 class="plan-task-detail-title">${escHtml(master.title)}</h2>
      <button type="button" class="md-header-btn plan-task-delete-master" data-action="delete-master"${disabledAttr}>删除计划</button>
    </div>
  `;
}

function renderAddSubControls(masterTaskId, writeUi) {
  const disabled = writeControlsDisabled(writeUi.writeState);
  const disabledAttr = disabled ? ' disabled' : '';
  if (writeUi.writeState === 'input' && writeUi.inputKind === 'add-sub') {
    return `
      <div class="plan-task-add-sub plan-task-write-input" data-write-zone="add-sub" data-master-id="${escHtml(masterTaskId)}">
        <label class="plan-task-write-label">
          <span>子任务标题</span>
          <input type="text" class="plan-task-write-field" data-input="add-sub-title" placeholder="必填" />
        </label>
        <div class="plan-task-write-actions">
          <button type="button" class="md-header-btn primary" data-action="submit-add-sub">添加</button>
          <button type="button" class="md-header-btn" data-action="cancel-input">取消</button>
        </div>
      </div>
    `;
  }
  return `
    <div class="plan-task-add-sub">
      <button type="button" class="md-header-btn plan-task-write-btn" data-action="add-sub" data-master-id="${escHtml(masterTaskId)}"${disabledAttr}>添加子任务</button>
    </div>
  `;
}

function renderSubDetailWithControls(master, selectedSubId, writeUi) {
  const subs = master.sub_tasks ?? [];
  const items = subs.map((sub) => renderSubRow(master, sub, selectedSubId, writeUi)).join('');
  const detailError =
    writeUi.writeError && writeUi.inputKind !== 'create-master' ? writeUi.writeError : '';
  return `
    <div class="plan-task-detail-body">
      ${renderDetailHeader(master, writeUi)}
      ${renderRefreshWarning(writeUi.refreshWarning, writeControlsDisabled(writeUi.writeState))}
      ${renderWriteError(detailError)}
      ${renderAddSubControls(master.master_task_id, writeUi)}
      <div class="plan-task-sub-list">${items}</div>
    </div>
  `;
}

function renderDetailEmpty() {
  return `
    <div class="plan-task-split-detail-empty plan-task-split-state">
      <p class="plan-task-split-state-title">选择左侧任务查看详情</p>
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

function renderSplitShell({ masterHtml, detailHtml }) {
  return `
    <div class="plan-task-split">
      <aside class="plan-task-split-master" aria-label="计划任务列表">${masterHtml}</aside>
      <section class="plan-task-split-detail" aria-label="任务详情">${detailHtml}</section>
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
  let writeState = 'idle';
  /** @type {'create-master' | 'add-sub' | null} */
  let inputKind = null;
  let writeError = '';
  let refreshWarning = '';
  let pendingDeleteMaster = false;

  container.innerHTML = '<div class="plan-task-split-loading">加载中…</div>';

  function getWriteUi() {
    return { writeState, inputKind, writeError, refreshWarning, pendingDeleteMaster };
  }

  function findMaster(id) {
    return masters.find((master) => master.master_task_id === id) ?? null;
  }

  function resetWriteInteraction({ keepRefreshWarning = false } = {}) {
    writeState = 'idle';
    inputKind = null;
    writeError = '';
    pendingDeleteMaster = false;
    if (!keepRefreshWarning) {
      refreshWarning = '';
    }
  }

  function paintWriteUi() {
    paint();
  }

  function showInputError(message, kind) {
    writeState = 'input';
    inputKind = kind;
    writeError = message;
    paintWriteUi();
  }

  function setWriteState(next, { error = '', nextInputKind = null, keepRefreshWarning = false } = {}) {
    writeState = next;
    writeError = error;
    if (next === 'input') {
      inputKind = nextInputKind;
      pendingDeleteMaster = false;
    }
    if (next === 'idle' && !keepRefreshWarning) {
      refreshWarning = '';
    }
    if (next !== 'input' && nextInputKind == null && next !== 'error') {
      inputKind = null;
    }
    paintWriteUi();
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
    return renderSubDetailWithControls(master, selectedSubId, getWriteUi());
  }

  function paint() {
    if (!masters.length && container.querySelector('.plan-task-split-error')) {
      return;
    }
    const writeUi = getWriteUi();
    const masterHtml = renderMasterPane(masters, selectedMasterId, writeUi);
    container.innerHTML = renderSplitShell({
      masterHtml,
      detailHtml: renderDetailPane(),
    });
  }

  async function reloadList({ afterWrite = false } = {}) {
    try {
      const entries = await loadPlanTasks();
      if (disposed) return;
      masters = entries;
      resolveSelection();
      if (afterWrite) {
        resetWriteInteraction({ keepRefreshWarning: false });
      }
      paint();
    } catch {
      if (disposed) return;
      if (afterWrite) {
        refreshWarning = REFRESH_WARNING_MSG;
        resetWriteInteraction({ keepRefreshWarning: true });
        paint();
        return;
      }
      masters = [];
      selectedMasterId = '';
      selectedSubId = '';
      resetWriteInteraction();
      container.innerHTML = renderSplitShell({
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

  async function refreshAfterWrite() {
    setWriteState('refresh');
    await reloadList({ afterWrite: true });
    if (writeState === 'refresh') {
      setWriteState('idle', { keepRefreshWarning: Boolean(refreshWarning) });
    }
  }

  async function runWriteAction(actionFn) {
    if (writeControlsDisabled(writeState)) return;
    setWriteState('writing');
    try {
      await actionFn();
      await refreshAfterWrite();
    } catch (err) {
      setWriteState('error', { error: err?.message || '操作失败' });
    }
  }

  const onClick = (event) => {
    const actionEl = event.target.closest('[data-action]');
    const action = actionEl?.dataset.action;

    if (action === 'retry-refresh') {
      event.preventDefault();
      if (writeControlsDisabled(writeState)) return;
      refreshWarning = '';
      void refresh();
      return;
    }

    if (action === 'create-master') {
      event.preventDefault();
      if (writeControlsDisabled(writeState)) return;
      setWriteState('input', { nextInputKind: 'create-master' });
      return;
    }

    if (action === 'cancel-input') {
      event.preventDefault();
      resetWriteInteraction({ keepRefreshWarning: Boolean(refreshWarning) });
      paintWriteUi();
      return;
    }

    if (action === 'submit-create-master') {
      event.preventDefault();
      if (writeControlsDisabled(writeState)) return;
      const zone = container.querySelector('[data-write-zone="create-master"]');
      const title = zone?.querySelector('[data-input="create-title"]')?.value?.trim() ?? '';
      const subTitlesRaw = zone?.querySelector('[data-input="create-subtitles"]')?.value ?? '';
      if (!title) {
        showInputError('请填写计划标题', 'create-master');
        return;
      }
      void runWriteAction(async () => {
        const result = await createPlanTask({ title, subTitles: parseSubTitles(subTitlesRaw) });
        const createdId = result?.master_task_id;
        if (createdId) {
          selectedMasterId = createdId;
          selectedSubId = '';
          deadLink = false;
        }
      });
      return;
    }

    if (action === 'delete-master') {
      event.preventDefault();
      if (writeControlsDisabled(writeState)) return;
      pendingDeleteMaster = true;
      writeError = '';
      paintWriteUi();
      return;
    }

    if (action === 'cancel-delete-master') {
      event.preventDefault();
      pendingDeleteMaster = false;
      writeError = '';
      paintWriteUi();
      return;
    }

    if (action === 'confirm-delete-master') {
      event.preventDefault();
      if (writeControlsDisabled(writeState) || !selectedMasterId) return;
      const masterTaskId = selectedMasterId;
      void runWriteAction(async () => {
        await deletePlanTask({ masterTaskId });
        selectedMasterId = '';
        selectedSubId = '';
        deadLink = false;
        pendingDeleteMaster = false;
        if (typeof navigate === 'function') {
          navigate('#/plan-tasks');
        }
      });
      return;
    }

    if (action === 'add-sub') {
      event.preventDefault();
      if (writeControlsDisabled(writeState)) return;
      setWriteState('input', { nextInputKind: 'add-sub' });
      return;
    }

    if (action === 'submit-add-sub') {
      event.preventDefault();
      if (writeControlsDisabled(writeState) || !selectedMasterId) return;
      const zone = container.querySelector('[data-write-zone="add-sub"]');
      const title = zone?.querySelector('[data-input="add-sub-title"]')?.value?.trim() ?? '';
      if (!title) {
        showInputError('请填写子任务标题', 'add-sub');
        return;
      }
      const masterTaskId = selectedMasterId;
      void runWriteAction(async () => {
        await addPlanSub({ masterTaskId, title });
      });
      return;
    }

    if (action === 'delete-sub') {
      event.preventDefault();
      event.stopPropagation();
      if (writeControlsDisabled(writeState) || !selectedMasterId) return;
      const subTaskId = actionEl?.dataset.subId;
      if (!subTaskId) return;
      const masterTaskId = selectedMasterId;
      void runWriteAction(async () => {
        await deletePlanSub({ masterTaskId, subTaskId });
      });
      return;
    }

    const masterBtn = event.target.closest('.plan-task-master-item');
    if (masterBtn?.dataset.masterId) {
      if (writeControlsDisabled(writeState)) return;
      selectedMasterId = masterBtn.dataset.masterId;
      const master = findMaster(selectedMasterId);
      const fallback = master ? pickDefaultSub(master) : null;
      selectedSubId = fallback?.sub_task_id ?? '';
      deadLink = false;
      resetWriteInteraction({ keepRefreshWarning: Boolean(refreshWarning) });
      paintWriteUi();
      if (typeof navigate === 'function' && selectedMasterId && selectedSubId) {
        navigate(buildDeepLink(selectedMasterId, selectedSubId));
      }
      return;
    }

    const subEl = event.target.closest('.plan-task-sub');
    if (subEl?.dataset.subId && selectedMasterId) {
      if (writeControlsDisabled(writeState)) return;
      selectedSubId = subEl.dataset.subId;
      deadLink = false;
      paintWriteUi();
      if (typeof navigate === 'function') {
        navigate(buildDeepLink(selectedMasterId, selectedSubId));
      }
    }
  };

  container.addEventListener('click', onClick);
  const disposeFocusRefresh = bindFocusRefresh(refresh);
  void refresh();

  function dispose() {
    disposed = true;
    disposeFocusRefresh();
    container.removeEventListener('click', onClick);
    container.innerHTML = '';
  }

  return { dispose, unmount: dispose, refresh };
}
