import { createApiClient, resolveReadDriver } from './apiClient.js';
import { escHtml } from './utils.js';

const UNAVAILABLE_MSG = '列表暂时不可用，请稍后重试';

function serviceError(data) {
  if (!data || typeof data !== 'object' || Array.isArray(data)) return null;
  if (!data.error) return null;
  const err = new Error(String(data.error));
  err.status = typeof data._status === 'number' ? data._status : 500;
  return err;
}

export async function loadAssistantPlanTasks() {
  const client = createApiClient(resolveReadDriver());
  const data = await client.getJson('/api/plan-tasks');
  const err = serviceError(data);
  if (err) throw err;
  return Array.isArray(data) ? data : [];
}

export function selectTop3ByCreatedAt(entries) {
  return [...entries]
    .sort((a, b) => {
      const aTime = Date.parse(a.created_at ?? '') || 0;
      const bTime = Date.parse(b.created_at ?? '') || 0;
      return bTime - aTime;
    })
    .slice(0, 3);
}

export function formatSubProgressSummary(master) {
  const subs = master.sub_tasks ?? [];
  const complete = subs.filter((sub) => sub.status === 'complete').length;
  const total = subs.length;
  return `${master.title} · ${complete}/${total} 完成`;
}

function pickSubForDeepLink(master) {
  const subs = master.sub_tasks ?? [];
  const incomplete = subs.find((sub) => sub.status !== 'complete');
  const sub = incomplete ?? subs[0];
  return sub?.sub_task_id ?? '';
}

export function buildDeepLink(masterId, subId) {
  const params = new URLSearchParams({ master: masterId, sub: subId });
  return `#/plan-tasks?${params.toString()}`;
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

function renderEmpty() {
  return `
    <div class="plan-task-assistant-empty plan-task-assistant-state">
      <p class="plan-task-assistant-state-title">暂无计划任务</p>
      <p class="plan-task-assistant-state-detail">通过 MCP 创建后，最新任务会出现在这里</p>
    </div>
  `;
}

function renderErrorEmpty(message = UNAVAILABLE_MSG) {
  return `
    <div class="plan-task-assistant-empty plan-task-assistant-state plan-task-assistant-state--error">
      <p class="plan-task-assistant-state-title">暂时无法加载</p>
      <p class="plan-task-assistant-state-detail">${escHtml(message)}</p>
    </div>
  `;
}

function renderTaskList(masters) {
  const items = masters
    .map((master) => {
      const subId = pickSubForDeepLink(master);
      const href = buildDeepLink(master.master_task_id, subId);
      const summary = formatSubProgressSummary(master);
      return `
        <li class="plan-task-assistant-item" data-master-id="${escHtml(master.master_task_id)}">
          <a class="plan-task-assistant-item-link" href="${escHtml(href)}" data-hash="${escHtml(href)}">
            <span class="plan-task-assistant-item-summary">${escHtml(summary)}</span>
          </a>
        </li>
      `;
    })
    .join('');
  return `<ul class="plan-task-assistant-list">${items}</ul>`;
}

function renderManageLink(showManage) {
  if (!showManage) return '';
  return `<button type="button" class="plan-task-assistant-manage-link">查看全部 →</button>`;
}

/**
 * @param {HTMLElement} root
 * @param {{ autoLoad?: boolean, navigate?: (hash: string) => void }} [opts]
 */
export function mountPlanTaskAssistant(root, opts = {}) {
  const { autoLoad = true, navigate } = opts;
  let disposed = false;
  let top3Masters = [];
  let refreshPromise = null;

  const showManage = typeof navigate === 'function';

  function renderCurrent() {
    const footer = renderManageLink(showManage);
    if (!top3Masters.length) {
      root.innerHTML = `${renderEmpty()}${footer}`;
      return;
    }
    root.innerHTML = `${renderTaskList(top3Masters)}${footer}`;
  }

  async function refreshAssistantTop3() {
    if (refreshPromise) {
      return refreshPromise;
    }

    refreshPromise = (async () => {
      if (!top3Masters.length) {
        root.innerHTML = '<div class="plan-task-assistant-loading">加载中…</div>';
      }

      try {
        const entries = await loadAssistantPlanTasks();
        if (disposed) return;
        top3Masters = selectTop3ByCreatedAt(entries);
        renderCurrent();
      } catch {
        if (disposed) return;
        top3Masters = [];
        root.innerHTML = `${renderErrorEmpty()}${renderManageLink(showManage)}`;
      }
    })().finally(() => {
      refreshPromise = null;
    });

    return refreshPromise;
  }

  const onClick = (event) => {
    if (event.target.closest('.plan-task-assistant-manage-link')) {
      if (typeof navigate === 'function') navigate('#/plan-tasks');
      return;
    }

    const itemLink = event.target.closest('.plan-task-assistant-item-link');
    if (itemLink) {
      event.preventDefault();
      const hash = itemLink.dataset.hash || itemLink.getAttribute('href');
      if (hash && typeof navigate === 'function') navigate(hash);
    }
  };

  root.addEventListener('click', onClick);
  const disposeFocusRefresh = bindFocusRefresh(refreshAssistantTop3);
  if (autoLoad) {
    void refreshAssistantTop3();
  }

  function dispose() {
    disposed = true;
    disposeFocusRefresh();
    root.removeEventListener('click', onClick);
    root.innerHTML = '';
  }

  return { dispose, refresh: refreshAssistantTop3 };
}

/**
 * Fixed bottom-right launcher offset above Read Later FAB (main window only).
 * @param {HTMLElement} [anchor]
 * @param {{ navigate?: (hash: string) => void }} [opts]
 */
export function mountPlanTaskAssistantWidget(anchor = document.body, opts = {}) {
  const { navigate } = opts;
  const widget = document.createElement('div');
  widget.className = 'pt-assistant-widget';
  widget.innerHTML = `
    <div class="pt-assistant-popover" hidden>
      <header class="pt-assistant-popover-header">
        <span class="pt-assistant-popover-title">计划任务助手</span>
        <button type="button" class="pt-assistant-close" aria-label="关闭">×</button>
      </header>
      <div class="pt-assistant-popover-body"></div>
    </div>
    <button type="button" class="pt-assistant-fab" aria-label="打开计划任务助手" aria-expanded="false" title="计划任务助手">
      <svg class="pt-assistant-fab-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
        <path fill="currentColor" d="M4 5.5A1.5 1.5 0 0 1 5.5 4h13A1.5 1.5 0 0 1 20 5.5v13a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 4 18.5v-13zM7 8h10v1.5H7V8zm0 3.5h10V13H7v-1.5zm0 3.5h6V16H7v-1z"/>
      </svg>
    </button>
  `;
  anchor.appendChild(widget);

  const popover = widget.querySelector('.pt-assistant-popover');
  const body = widget.querySelector('.pt-assistant-popover-body');
  const fab = widget.querySelector('.pt-assistant-fab');
  const closeBtn = widget.querySelector('.pt-assistant-close');

  let panel = null;
  let open = false;

  function setOpen(next) {
    open = next;
    popover.hidden = !open;
    fab.setAttribute('aria-expanded', String(open));
    fab.classList.toggle('pt-assistant-fab--active', open);
    if (!open) return;
    if (!panel) {
      const panelNavigate =
        typeof navigate === 'function'
          ? (hash) => {
              setOpen(false);
              navigate(hash);
            }
          : undefined;
      panel = mountPlanTaskAssistant(body, {
        autoLoad: true,
        navigate: panelNavigate,
      });
      return;
    }
    void panel.refresh();
  }

  fab.addEventListener('click', (event) => {
    event.stopPropagation();
    setOpen(!open);
  });
  closeBtn.addEventListener('click', (event) => {
    event.stopPropagation();
    setOpen(false);
  });

  const onDocClick = (event) => {
    if (!open) return;
    if (widget.contains(event.target)) return;
    setOpen(false);
  };
  document.addEventListener('click', onDocClick, true);

  function dispose() {
    document.removeEventListener('click', onDocClick, true);
    panel?.dispose();
    widget.remove();
  }

  return { dispose, setOpen };
}

const bootstrapRoot = document.getElementById('plan-task-assistant-root');
if (bootstrapRoot) {
  mountPlanTaskAssistantWidget(document.body);
}
