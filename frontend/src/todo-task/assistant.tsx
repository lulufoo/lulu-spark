// @ts-nocheck
import { flushSync } from 'react-dom';
import { createRoot } from 'react-dom/client';
import { createApiClient, resolveReadDriver } from '../host/apiClient.ts';
import { formatTodoTaskStatus } from './index.ts';

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

function EmptyState() {
  return (
    <div className="todo-task-assistant-empty todo-task-assistant-state">
      <p className="todo-task-assistant-state-title">No todos yet</p>
      <p className="todo-task-assistant-state-detail">After creating via MCP, latest tasks appear here</p>
    </div>
  );
}

function ErrorEmpty({ message = UNAVAILABLE_MSG }) {
  return (
    <div className="todo-task-assistant-empty todo-task-assistant-state todo-task-assistant-state--error">
      <p className="todo-task-assistant-state-title">Temporarily unavailable</p>
      <p className="todo-task-assistant-state-detail">{message}</p>
    </div>
  );
}

function TaskList({ masters }) {
  return (
    <ul className="todo-task-assistant-list">
      {masters.map((master) => {
        const subId = pickSubForDeepLink(master);
        const href = buildDeepLink(master.master_task_id, subId);
        const summary = formatSubProgressSummary(master);
        const status = masterStatusClass(master.status);
        const statusLabel = formatTodoTaskStatus(status);
        return (
          <li
            key={master.master_task_id}
            className={`todo-task-assistant-item todo-task-assistant-item--${status}`}
            data-master-id={master.master_task_id}
          >
            <a className="todo-task-assistant-item-link" href={href} data-hash={href}>
              <span className="todo-task-assistant-item-status">{statusLabel}</span>
              <span className="todo-task-assistant-item-summary">{summary}</span>
            </a>
          </li>
        );
      })}
    </ul>
  );
}

function ManageLink() {
  return (
    <button type="button" className="todo-task-assistant-manage-link">
      View all →
    </button>
  );
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
  let reactRoot = createRoot(root);
  let errorMessage = null;

  const showManage = typeof navigate === 'function';

  function paint(loading = false) {
    if (!reactRoot || disposed) return;
    flushSync(() => {
      if (loading) {
        reactRoot.render(<div className="todo-task-assistant-loading">Loading…</div>);
        return;
      }
      if (errorMessage) {
        reactRoot.render(
          <>
            <ErrorEmpty message={errorMessage} />
            {showManage ? <ManageLink /> : null}
          </>,
        );
        return;
      }
      if (!top3Masters.length) {
        reactRoot.render(
          <>
            <EmptyState />
            {showManage ? <ManageLink /> : null}
          </>,
        );
        return;
      }
      reactRoot.render(
        <>
          <TaskList masters={top3Masters} />
          {showManage ? <ManageLink /> : null}
        </>,
      );
    });
  }

  async function refreshAssistantTop3() {
    if (refreshPromise) {
      return refreshPromise;
    }

    refreshPromise = (async () => {
      if (!top3Masters.length) {
        paint(true);
      }

      try {
        const entries = await loadAssistantTodoTasks();
        if (disposed) return;
        top3Masters = selectTop3ByCreatedAt(entries);
        errorMessage = null;
        paint();
      } catch {
        if (disposed) return;
        top3Masters = [];
        errorMessage = UNAVAILABLE_MSG;
        paint();
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
    flushSync(() => {
      reactRoot?.unmount();
    });
    reactRoot = null;
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
