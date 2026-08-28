import type { MouseEvent } from 'react';
import { flushSync } from 'react-dom';
import { createRoot, type Root } from 'react-dom/client';
import { createApiClient, resolveReadDriver } from '../host/apiClient.ts';

const UNAVAILABLE_MSG = 'List temporarily unavailable. Please try again later';

export type ReadLaterFilter = 'all' | 'unread';

type ReadLaterEntry = {
  id: string;
  title?: string;
  url: string;
  saved_at?: string;
  read?: boolean;
};

function getTauriInvoke() {
  if (typeof window === 'undefined') return null;
  const invoke =
    window.__TAURI__?.core?.invoke || window.__TAURI_INTERNALS__?.invoke;
  return typeof invoke === 'function' ? invoke : null;
}

function getTauriOpener() {
  if (typeof window === 'undefined') return null;
  const opener = window.__TAURI__?.opener;
  const openUrl = opener?.openUrl;
  return typeof openUrl === 'function' ? openUrl.bind(opener) : null;
}

export async function openExternalUrl(url: string) {
  const openUrl = getTauriOpener();
  if (!openUrl) {
    throw new Error('Tauri opener unavailable');
  }
  await openUrl(url);
}

function serviceError(data: unknown) {
  if (!data || typeof data !== 'object' || Array.isArray(data)) return null;
  const rec = data as { error?: unknown; _status?: number };
  if (!rec.error) return null;
  const err = new Error(String(rec.error)) as Error & { status?: number };
  err.status = typeof rec._status === 'number' ? rec._status : 500;
  return err;
}

function httpStatusError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  const match = message.match(/^HTTP (\d+)/);
  if (!match) return null;
  const err = new Error(message) as Error & { status?: number };
  err.status = Number(match[1]);
  return err;
}

function formatSavedAt(savedAt: string | undefined) {
  if (!savedAt) return '';
  const date = new Date(savedAt);
  if (Number.isNaN(date.getTime())) return savedAt;
  return date.toLocaleString('zh-CN', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export async function loadReadLaterEntries() {
  const mode = resolveReadDriver();
  const client = createApiClient(resolveReadDriver(mode));
  try {
    const data = await client.getJson('/api/read-later');
    const unavailable = serviceError(data);
    if (unavailable) throw unavailable;
    return Array.isArray(data) ? data : [];
  } catch (error) {
    const httpErr = httpStatusError(error);
    if (httpErr) throw httpErr;
    throw error;
  }
}

export async function markEntryRead(id: string) {
  const invoke = getTauriInvoke();
  if (!invoke) {
    throw new Error('Tauri invoke unavailable');
  }
  const result = await invoke('mark_read_later', { id, read: true });
  const unavailable = serviceError(result);
  if (unavailable) throw unavailable;
}

export async function deleteReadLaterEntry(id: string) {
  const invoke = getTauriInvoke();
  if (!invoke) {
    throw new Error('Tauri invoke unavailable');
  }
  const result = await invoke('delete_read_later', { id });
  const unavailable = serviceError(result);
  if (unavailable) throw unavailable;
}

export function bindFocusRefresh(refresh: () => unknown) {
  const runRefresh = () => {
    void refresh();
  };
  const onVisibility = () => {
    if (document.visibilityState === 'visible') {
      runRefresh();
    }
  };
  window.addEventListener('focus', runRefresh);
  document.addEventListener('visibilitychange', onVisibility);
  return () => {
    window.removeEventListener('focus', runRefresh);
    document.removeEventListener('visibilitychange', onVisibility);
  };
}

function sortEntries(entries: ReadLaterEntry[]) {
  return [...entries].sort((a, b) => {
    const aTime = Date.parse(a.saved_at ?? '') || 0;
    const bTime = Date.parse(b.saved_at ?? '') || 0;
    return bTime - aTime;
  });
}

function filterEntries(entries: ReadLaterEntry[], filter: ReadLaterFilter) {
  if (filter === 'unread') {
    return sortEntries(entries.filter((entry) => !entry.read));
  }
  return sortEntries(entries);
}

function emptyMessage(filter: ReadLaterFilter) {
  return filter === 'unread' ? 'No items to read later' : 'No items';
}

function ReadLaterTabs({ filter }: { filter: ReadLaterFilter }) {
  return (
    <div className="read-later-tabs" role="tablist" aria-label="Filter read-later">
      <button
        type="button"
        className={`read-later-tab${filter === 'all' ? ' read-later-tab--active' : ''}`}
        data-filter="all"
        role="tab"
        aria-selected={filter === 'all'}
      >
        All
      </button>
      <button
        type="button"
        className={`read-later-tab${filter === 'unread' ? ' read-later-tab--active' : ''}`}
        data-filter="unread"
        role="tab"
        aria-selected={filter === 'unread'}
      >
        Unread
      </button>
    </div>
  );
}

function LinkEntry({ entry }: { entry: ReadLaterEntry }) {
  const title = entry.title || entry.url;
  const savedAt = formatSavedAt(entry.saved_at);
  const readClass = entry.read ? ' read-later-item--read' : ' read-later-item--unread';
  return (
    <li className={`read-later-item${readClass}`} data-entry-id={entry.id}>
      <a className="read-later-link" href={entry.url} data-url={entry.url}>
        <span className="read-later-link-title">{title}</span>
        {savedAt ? <span className="read-later-link-meta">{savedAt}</span> : null}
      </a>
      <button type="button" className="read-later-delete" data-id={entry.id} aria-label="Delete">
        ×
      </button>
    </li>
  );
}

function UnavailableBanner({ message = UNAVAILABLE_MSG }: { message?: string }) {
  return <div className="read-later-unavailable">{message}</div>;
}

function ListBody({
  entries,
  showUnavailable = false,
  message = UNAVAILABLE_MSG,
  filter = 'unread',
}: {
  entries: ReadLaterEntry[];
  showUnavailable?: boolean;
  message?: string;
  filter?: ReadLaterFilter;
}) {
  const displayEntries = filterEntries(entries, filter);
  if (!displayEntries.length) {
    return (
      <>
        <div className="read-later-empty">{emptyMessage(filter)}</div>
        {showUnavailable ? <UnavailableBanner message={message} /> : null}
      </>
    );
  }
  return (
    <>
      {showUnavailable ? <UnavailableBanner message={message} /> : null}
      <ul className="read-later-list">
        {displayEntries.map((entry) => (
          <LinkEntry key={entry.id} entry={entry} />
        ))}
      </ul>
    </>
  );
}

export function renderUnavailableState(
  container: HTMLElement,
  {
    mode,
    message = UNAVAILABLE_MSG,
    entries = [],
    filter = 'unread',
    showTabs = false,
  }: {
    mode?: string;
    message?: string;
    entries?: ReadLaterEntry[];
    filter?: ReadLaterFilter;
    showTabs?: boolean;
  } = {},
) {
  const root = createRoot(container);
  const view =
    mode === 'empty' ? (
      <>
        {showTabs ? <ReadLaterTabs filter={filter} /> : null}
        <div className={showTabs ? 'read-later-list-host' : undefined}>
          <div className="read-later-empty">{emptyMessage(filter)}</div>
          <UnavailableBanner message={message} />
        </div>
      </>
    ) : mode === 'retained' ? (
      <>
        {showTabs ? <ReadLaterTabs filter={filter} /> : null}
        <div className={showTabs ? 'read-later-list-host' : undefined}>
          <ListBody entries={entries} showUnavailable message={message} filter={filter} />
        </div>
      </>
    ) : null;
  flushSync(() => {
    root.render(view);
  });
}

/**
 * @param {HTMLElement} container
 * @param {{ showTabs?: boolean, initialFilter?: ReadLaterFilter }} [opts]
 */
export function mountReadLaterList(
  container: HTMLElement,
  opts: { showTabs?: boolean; initialFilter?: ReadLaterFilter } = {},
) {
  const { showTabs = false, initialFilter = 'unread' } = opts;
  let filter: ReadLaterFilter = initialFilter;
  let disposed = false;
  let lastSuccessfulEntries: ReadLaterEntry[] | null = null;
  let refreshPromise: Promise<void> | null = null;
  let root: Root | null = createRoot(container);
  let showUnavailable = false;
  let unavailableMessage = UNAVAILABLE_MSG;

  function paint(loading = false) {
    if (!root || disposed) return;
    flushSync(() => {
      if (loading && lastSuccessfulEntries === null) {
        root!.render(<div className="read-later-loading">Loading…</div>);
        return;
      }
      if (lastSuccessfulEntries === null) {
        root!.render(<div className="read-later-loading">Loading…</div>);
        return;
      }
      root!.render(
        <div onClick={onClick}>
          {showTabs ? <ReadLaterTabs filter={filter} /> : null}
          <div className={showTabs ? 'read-later-list-host' : undefined}>
            <ListBody
              entries={lastSuccessfulEntries}
              showUnavailable={showUnavailable}
              message={unavailableMessage}
              filter={filter}
            />
          </div>
        </div>,
      );
    });
  }

  async function refresh() {
    if (refreshPromise) {
      return refreshPromise;
    }

    refreshPromise = (async () => {
      const hasSnapshot = lastSuccessfulEntries !== null;
      if (!hasSnapshot) {
        paint(true);
      }

      try {
        const entries = (await loadReadLaterEntries()) as ReadLaterEntry[];
        if (disposed) return;
        lastSuccessfulEntries = entries;
        showUnavailable = false;
        paint();
      } catch {
        if (disposed) return;
        showUnavailable = true;
        if (lastSuccessfulEntries !== null) {
          paint();
        } else {
          lastSuccessfulEntries = [];
          paint();
        }
      }
    })().finally(() => {
      refreshPromise = null;
    });

    return refreshPromise;
  }

  const disposeFocusRefresh = bindFocusRefresh(refresh);

  const onClick = (event: MouseEvent<HTMLElement>) => {
    const target = event.target as HTMLElement;
    const tab = target.closest('.read-later-tab');
    if (tab && 'dataset' in tab && (tab as HTMLElement).dataset.filter) {
      filter = (tab as HTMLElement).dataset.filter === 'all' ? 'all' : 'unread';
      paint();
      return;
    }

    const deleteBtn = target.closest('.read-later-delete') as HTMLButtonElement | null;
    if (deleteBtn) {
      event.preventDefault();
      const { id } = deleteBtn.dataset;
      if (!id) return;
      const item = deleteBtn.closest('.read-later-item');
      deleteBtn.disabled = true;
      item?.querySelector('.read-later-action-error')?.remove();
      void deleteReadLaterEntry(id)
        .then(() => {
          if (disposed) return;
          if (lastSuccessfulEntries) {
            lastSuccessfulEntries = lastSuccessfulEntries.filter((entry) => entry.id !== id);
          }
          paint();
        })
        .catch((err: { message?: string }) => {
          if (disposed || !item) return;
          deleteBtn.disabled = false;
          const errEl = document.createElement('div');
          errEl.className = 'read-later-action-error';
          errEl.textContent = err?.message || 'Delete failed';
          item.appendChild(errEl);
        });
      return;
    }

    const link = target.closest('.read-later-link') as HTMLAnchorElement | null;
    if (!link) return;
    event.preventDefault();
    const item = link.closest('.read-later-item');
    const id = (item as HTMLElement | null)?.dataset?.entryId;
    const url = link.dataset.url || link.getAttribute('href');
    if (!url) return;

    item?.querySelector('.read-later-action-error')?.remove();
    void openExternalUrl(url)
      .then(() => {
        if (disposed || !id) return;
        const entry = lastSuccessfulEntries?.find((e) => e.id === id);
        if (entry?.read) return;
        void markEntryRead(id)
          .then(() => {
            if (disposed) return;
            if (lastSuccessfulEntries) {
              lastSuccessfulEntries = lastSuccessfulEntries.map((e) =>
                e.id === id ? { ...e, read: true } : e,
              );
            }
            paint();
          })
          .catch((err: { message?: string }) => {
            if (disposed || !item) return;
            const errEl = document.createElement('div');
            errEl.className = 'read-later-action-error';
            errEl.textContent = err?.message || 'Failed to mark as read';
            item.appendChild(errEl);
          });
      })
      .catch((err: { message?: string }) => {
        if (disposed || !item) return;
        const errEl = document.createElement('div');
        errEl.className = 'read-later-action-error';
        errEl.textContent = err?.message || 'Failed to open link';
        item.appendChild(errEl);
      });
  };

  paint(true);
  void refresh();

  function dispose() {
    disposed = true;
    disposeFocusRefresh();
    flushSync(() => {
      root?.unmount();
    });
    root = null;
    container.innerHTML = '';
  }

  return { dispose, unmount: dispose, refresh };
}
