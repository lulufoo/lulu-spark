import { createApiClient, resolveReadDriver } from '../apiClient.js';
import { escHtml } from '../utils.js';

const UNAVAILABLE_MSG = '列表暂时不可用，请稍后重试';

function serviceError(data) {
  if (!data || typeof data !== 'object' || Array.isArray(data)) return null;
  if (!data.error) return null;
  const err = new Error(String(data.error));
  err.status = typeof data._status === 'number' ? data._status : 500;
  return err;
}

export async function loadPlanTasks() {
  const client = createApiClient(resolveReadDriver());
  const data = await client.getJson('/api/plan-tasks');
  const err = serviceError(data);
  if (err) throw err;
  return Array.isArray(data) ? data : [];
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

function renderLinkedArchives(linkedArchiveIds) {
  if (!linkedArchiveIds?.length) return '';
  return `<div class="plan-task-sub-archives">关联归档：${escHtml(linkedArchiveIds.join(', '))}</div>`;
}

function renderSubRow(master, sub, selectedSubId) {
  const selected = sub.sub_task_id === selectedSubId ? ' plan-task-sub--selected' : '';
  const copyText = copySubIdPair(master.master_task_id, sub.sub_task_id);
  const title = sub.title || sub.sub_task_id;
  return `
    <article data-sub-id="${escHtml(sub.sub_task_id)}" class="plan-task-sub${selected}">
      <header class="plan-task-sub-header">
        <h3 class="plan-task-sub-title">${escHtml(title)}</h3>
        <span class="plan-task-sub-status plan-task-sub-status--${escHtml(sub.status)}">${escHtml(sub.status)}</span>
      </header>
      <div class="plan-task-sub-copy">${escHtml(copyText)}</div>
      ${renderLinkedArchives(sub.linked_archive_ids)}
    </article>
  `;
}

export function renderSubDetail(master, selectedSubId) {
  const subs = master.sub_tasks ?? [];
  const items = subs.map((sub) => renderSubRow(master, sub, selectedSubId)).join('');
  return `
    <div class="plan-task-detail-body">
      <h2 class="plan-task-detail-title">${escHtml(master.title)}</h2>
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

  container.innerHTML = '<div class="plan-task-split-loading">加载中…</div>';

  function findMaster(id) {
    return masters.find((master) => master.master_task_id === id) ?? null;
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
    return renderSubDetail(master, selectedSubId);
  }

  function paint() {
    if (!masters.length && container.querySelector('.plan-task-split-error')) {
      return;
    }
    const masterHtml = masters.length
      ? renderMasterList(masters, selectedMasterId)
      : '<div class="plan-task-split-state"><p class="plan-task-split-state-title">暂无计划任务</p></div>';
    container.innerHTML = renderSplitShell({
      masterHtml,
      detailHtml: renderDetailPane(),
    });
  }

  async function refresh() {
    if (refreshPromise) {
      return refreshPromise;
    }

    refreshPromise = (async () => {
      try {
        const entries = await loadPlanTasks();
        if (disposed) return;
        masters = entries;
        resolveSelection();
        paint();
      } catch {
        if (disposed) return;
        masters = [];
        selectedMasterId = '';
        selectedSubId = '';
        container.innerHTML = renderSplitShell({
          masterHtml: '<div class="plan-task-split-state"></div>',
          detailHtml: renderErrorEmpty(),
        });
      }
    })().finally(() => {
      refreshPromise = null;
    });

    return refreshPromise;
  }

  const onClick = (event) => {
    const masterBtn = event.target.closest('.plan-task-master-item');
    if (masterBtn?.dataset.masterId) {
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
      selectedSubId = subEl.dataset.subId;
      deadLink = false;
      paint();
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
