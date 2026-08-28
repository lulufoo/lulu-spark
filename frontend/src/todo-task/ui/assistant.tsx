import { useCallback, useEffect, useRef, useState } from 'react';
import { flushSync } from 'react-dom';
import { createRoot } from 'react-dom/client';
import { formatTodoTaskStatus } from '../index.ts';
import {
  buildDeepLink,
  formatSubProgressSummary,
  loadAssistantTodoTasks,
  selectTop3ByCreatedAt,
} from '../commands/assistant.ts';
import type { TodoMaster, TodoSub } from '../state/types.ts';

export {
  buildDeepLink,
  formatSubProgressSummary,
  loadAssistantTodoTasks,
  selectTop3ByCreatedAt,
} from '../commands/assistant.ts';

const UNAVAILABLE_MSG = 'List temporarily unavailable. Please try again later.';

function masterStatusClass(status?: string) {
  return status === 'complete' || status === 'abandoned' ? status : 'incomplete';
}

function pickSubForDeepLink(master: TodoMaster) {
  const subs = master.sub_tasks ?? [];
  const incomplete = subs.find((sub: TodoSub) => sub.status !== 'complete');
  const sub = incomplete ?? subs[0];
  return sub?.sub_task_id ?? '';
}

function bindFocusRefresh(refresh: () => Promise<void> | void) {
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

function TaskList({
  masters,
  navigate,
}: {
  masters: TodoMaster[];
  navigate?: (hash: string) => void;
}) {
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
            <a
              className="todo-task-assistant-item-link"
              href={href}
              data-hash={href}
              onClick={(event) => {
                if (typeof navigate !== 'function') return;
                event.preventDefault();
                navigate(href);
              }}
            >
              <span className="todo-task-assistant-item-status">{statusLabel}</span>
              <span className="todo-task-assistant-item-summary">{summary}</span>
            </a>
          </li>
        );
      })}
    </ul>
  );
}

function ManageLink({ onManage }: { onManage: () => void }) {
  return (
    <button type="button" className="todo-task-assistant-manage-link" onClick={onManage}>
      View all →
    </button>
  );
}

export function TodoTaskAssistant({
  autoLoad = true,
  navigate,
  handleRef,
}: {
  autoLoad?: boolean;
  navigate?: (hash: string) => void;
  handleRef?: { current: { refresh: () => Promise<void> } };
}) {
  const [loading, setLoading] = useState(false);
  const [masters, setMasters] = useState<TodoMaster[]>([]);
  const [error, setError] = useState<string | null>(null);
  const refreshPromise = useRef<Promise<void> | null>(null);
  const mastersRef = useRef(masters);
  mastersRef.current = masters;
  const showManage = typeof navigate === 'function';

  const refresh = useCallback(async () => {
    if (refreshPromise.current) return refreshPromise.current;
    refreshPromise.current = (async () => {
      if (!mastersRef.current.length) setLoading(true);
      try {
        const entries = await loadAssistantTodoTasks();
        setMasters(selectTop3ByCreatedAt(entries));
        setError(null);
      } catch {
        setMasters([]);
        setError(UNAVAILABLE_MSG);
      } finally {
        setLoading(false);
      }
    })().finally(() => {
      refreshPromise.current = null;
    });
    return refreshPromise.current;
  }, []);

  useEffect(() => {
    if (handleRef) handleRef.current = { refresh };
  }, [handleRef, refresh]);

  useEffect(() => {
    const dispose = bindFocusRefresh(refresh);
    if (autoLoad) void refresh();
    return dispose;
  }, [autoLoad, refresh]);

  const manage = showManage ? (
    <ManageLink
      onManage={() => {
        navigate('#/todo-tasks');
      }}
    />
  ) : null;

  if (loading) {
    return <div className="todo-task-assistant-loading">Loading…</div>;
  }
  if (error) {
    return (
      <>
        <ErrorEmpty message={error} />
        {manage}
      </>
    );
  }
  if (!masters.length) {
    return (
      <>
        <EmptyState />
        {manage}
      </>
    );
  }
  return (
    <>
      <TaskList masters={masters} navigate={navigate} />
      {manage}
    </>
  );
}

/**
 * @param {HTMLElement} root
 * @param {{ autoLoad?: boolean, navigate?: (hash: string) => void }} [opts]
 */
export function mountTodoTaskAssistant(
  root: HTMLElement,
  opts: { autoLoad?: boolean; navigate?: (hash: string) => void } = {},
) {
  const handleRef = { current: { refresh: () => Promise.resolve() } };
  const reactRoot = createRoot(root);
  flushSync(() => {
    reactRoot.render(<TodoTaskAssistant {...opts} handleRef={handleRef} />);
  });

  return {
    dispose() {
      flushSync(() => {
        reactRoot.unmount();
      });
      root.innerHTML = '';
    },
    refresh() {
      return handleRef.current.refresh();
    },
  };
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
    mount(
      slotEl: HTMLElement,
      ctx: { host?: { navigate?: (hash: string) => void } } = {},
    ) {
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
