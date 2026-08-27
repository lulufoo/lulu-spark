import { createApiClient, resolveReadDriver } from './apiClient.js';
import { formatTodoTaskStatus } from './todo-task/index.js';
import { escHtml } from './utils.js';

const UNAVAILABLE_MSG = 'List temporarily unavailable. Please try again later.';

function masterStatusClass(status) {
  return status === 'complete' || status === 'abandoned' ? status : 'incomplete';
}

function serviceError(data) {
  if (!data || typeof data !== 'object' || Array.isArray(data)) return null;
  if (!data.error) return null;
  const err = new Error(String(data.error));
  err.status = typeof data._status === 'number' ? data._status : 500;
  return err;
}

export async function loadAssistantTodoTasks() {
  const mode = resolveReadDriver();
  const client = createApiClient(resolveReadDriver(mode));
  const data = await client.getJson('/api/todo-tasks');
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
  return `${master.title} · ${complete}/${total} complete`;
}

function pickSubForDeepLink(master) {
  const subs = master.sub_tasks ?? [];
  const incomplete = subs.find((sub) => sub.status !== 'complete');
  const sub = incomplete ?? subs[0];
  return sub?.sub_task_id ?? '';
}

export function buildDeepLink(masterId, subId) {
  const params = new URLSearchParams({ master: masterId, sub: subId });
  return `#/todo-tasks?${params.toString()}`;
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
    <div class="todo-task-assistant-empty todo-task-assistant-state">
      <p class="todo-task-assistant-state-title">No todos yet</p>
      <p class="todo-task-assistant-state-detail">After creating via MCP, latest tasks appear here</p>
    </div>
  `;
}

function renderErrorEmpty(message = UNAVAILABLE_MSG) {
  return `
    <div class="todo-task-assistant-empty todo-task-assistant-state todo-task-assistant-state--error">
      <p class="todo-task-assistant-state-title">Temporarily unavailable</p>
      <p class="todo-task-assistant-state-detail">${escHtml(message)}</p>
    </div>
  `;
}

function renderTaskList(masters) {
  const items = masters
    .map((master) => {
      const subId = pickSubForDeepLink(master);
      const href = buildDeepLink(master.master_task_id, subId);
      const summary = formatSubProgressSummary(master);
      const status = masterStatusClass(master.status);
      const statusLabel = formatTodoTaskStatus(status);
      return `
        <li class="todo-task-assistant-item todo-task-assistant-item--${escHtml(status)}" data-master-id="${escHtml(master.master_task_id)}">
          <a class="todo-task-assistant-item-link" href="${escHtml(href)}" data-hash="${escHtml(href)}">
            <span class="todo-task-assistant-item-status">${escHtml(statusLabel)}</span>
            <span class="todo-task-assistant-item-summary">${escHtml(summary)}</span>
          </a>
        </li>
      `;
    })
    .join('');
  return `<ul class="todo-task-assistant-list">${items}</ul>`;
}

function renderManageLink(showManage) {
  if (!showManage) return '';
  return `<button type="button" class="todo-task-assistant-manage-link">View all →</button>`;
}

/**
 * @param {HTMLElement} root
 * @param {{ autoLoad?: boolean, navigate?: (hash: string) => void }} [opts]
 */
export function mountTodoTaskAssistant(root, opts = {}) {
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
        root.innerHTML = '<div class="todo-task-assistant-loading">Loading…</div>';
      }

      try {
        const entries = await loadAssistantTodoTasks();
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
    if (event.target.closest('.todo-task-assistant-manage-link')) {
      if (typeof navigate === 'function') navigate('#/todo-tasks');
      return;
    }

    const itemLink = event.target.closest('.todo-task-assistant-item-link');
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
 * t4 Boundary Out: FAB Top3 stays non-chat list widget — not Present/Set/execute.
 * De-embed does not upgrade this surface into a chat entry.
 */
export const TODO_TASK_ASSISTANT_FAB_CHAT_DISABLED = true;

/** Brand / a11y labels for the Todos content region (shell owns overlay title). */
export const TODO_TASK_CONTENT_LABEL = 'Open Todos';
export const TODO_TASK_CONTENT_TITLE = 'Todos';

/**
 * Todos content adapter for the home-entry shell content slot.
 * Shell owns overlay chrome; this module only paints task content into the slot.
 * @returns {{ mount: (slotEl: HTMLElement, ctx?: { host?: { navigate?: Function } }) => { unmount: () => void } }}
 */
export function createTodoTaskContentAdapter() {
  return {
    /**
     * @param {HTMLElement} slotEl
     * @param {{ host?: { navigate?: (hash: string) => void } }} [ctx]
     */
    mount(slotEl, ctx = {}) {
      const host = ctx.host ?? {};
      slotEl.setAttribute('aria-label', TODO_TASK_CONTENT_LABEL);
      slotEl.setAttribute('title', TODO_TASK_CONTENT_TITLE);
      const panel = mountTodoTaskAssistant(slotEl, {
        autoLoad: true,
        navigate: typeof host.navigate === 'function' ? host.navigate : undefined,
      });
      return {
        unmount() {
          panel.dispose();
          slotEl.removeAttribute('aria-label');
          slotEl.removeAttribute('title');
        },
      };
    },
  };
}

/**
 * @param {HTMLElement} slotEl
 * @param {{ navigate?: (hash: string) => void }} [host]
 * @returns {{ unmount: () => void }}
 */
export function mountTodoTaskContent(slotEl, host) {
  return createTodoTaskContentAdapter().mount(slotEl, { host });
}
