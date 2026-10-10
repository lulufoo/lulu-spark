import { useEffect, useRef, useState, type MouseEvent } from 'react';
import { flushSync } from 'react-dom';
import { createRoot, type Root } from 'react-dom/client';
import { ViewerHeaderIcon } from '../../shared/viewer-header-icons.tsx';
import { openReadLaterInChat } from '../commands/open-in-chat.ts';
import {
  bindFocusRefresh,
  deleteReadLaterEntry,
  loadReadLaterEntries,
  markEntryRead,
  openExternalUrl,
} from '../commands/list.ts';
import { emptyMessage, filterEntries } from '../state/selectors.ts';
import type { ReadLaterEntry, ReadLaterFilter } from '../state/types.ts';

export type { ReadLaterFilter };
export {
  bindFocusRefresh,
  deleteReadLaterEntry,
  loadReadLaterEntries,
  markEntryRead,
  openExternalUrl,
};

const UNAVAILABLE_MSG = 'List temporarily unavailable. Please try again later';

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
      <div className="read-later-item-actions">
        <button
          type="button"
          className="read-later-open-in-chat"
          data-id={entry.id}
          aria-label="Open in chat"
          title="Open in chat"
        >
          <ViewerHeaderIcon name="chat" filled />
        </button>
        <button type="button" className="read-later-delete" data-id={entry.id} aria-label="Delete">
          ×
        </button>
      </div>
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

export function ReadLaterList({
  showTabs = false,
  initialFilter = 'unread',
}: {
  showTabs?: boolean;
  initialFilter?: ReadLaterFilter;
}) {
  const [filter, setFilter] = useState<ReadLaterFilter>(initialFilter);
  const [entries, setEntries] = useState<ReadLaterEntry[] | null>(null);
  const [showUnavailable, setShowUnavailable] = useState(false);
  const refreshPromise = useRef<Promise<void> | null>(null);
  const disposed = useRef(false);

  async function refresh() {
    if (refreshPromise.current) return refreshPromise.current;
    refreshPromise.current = (async () => {
      try {
        const next = (await loadReadLaterEntries()) as ReadLaterEntry[];
        if (disposed.current) return;
        setEntries(next);
        setShowUnavailable(false);
      } catch {
        if (disposed.current) return;
        setShowUnavailable(true);
        setEntries((prev) => (prev === null ? [] : prev));
      }
    })().finally(() => {
      refreshPromise.current = null;
    });
    return refreshPromise.current;
  }

  useEffect(() => {
    disposed.current = false;
    const disposeFocus = bindFocusRefresh(refresh);
    void refresh();
    return () => {
      disposed.current = true;
      disposeFocus();
    };
  }, []);

  const onClick = (event: MouseEvent<HTMLElement>) => {
    const target = event.target as HTMLElement;
    const tab = target.closest('.read-later-tab');
    if (tab && 'dataset' in tab && (tab as HTMLElement).dataset.filter) {
      setFilter((tab as HTMLElement).dataset.filter === 'all' ? 'all' : 'unread');
      return;
    }

    const chatBtn = target.closest('.read-later-open-in-chat') as HTMLButtonElement | null;
    if (chatBtn) {
      event.preventDefault();
      const { id } = chatBtn.dataset;
      const item = chatBtn.closest('.read-later-item');
      const entry = id ? entries?.find((row) => row.id === id) : undefined;
      if (!entry) return;
      item?.querySelector('.read-later-action-error')?.remove();
      void openReadLaterInChat(entry, chatBtn).catch((err: { message?: string }) => {
        if (disposed.current || !item) return;
        const errEl = document.createElement('div');
        errEl.className = 'read-later-action-error';
        errEl.textContent = err?.message || 'Failed to open in chat';
        item.appendChild(errEl);
      });
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
          if (disposed.current) return;
          setEntries((prev) => (prev ? prev.filter((entry) => entry.id !== id) : prev));
        })
        .catch((err: { message?: string }) => {
          if (disposed.current || !item) return;
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
        if (disposed.current || !id) return;
        const entry = entries?.find((e) => e.id === id);
        if (entry?.read) return;
        void markEntryRead(id)
          .then(() => {
            if (disposed.current) return;
            setEntries((prev) =>
              prev ? prev.map((e) => (e.id === id ? { ...e, read: true } : e)) : prev,
            );
          })
          .catch((err: { message?: string }) => {
            if (disposed.current || !item) return;
            const errEl = document.createElement('div');
            errEl.className = 'read-later-action-error';
            errEl.textContent = err?.message || 'Failed to mark as read';
            item.appendChild(errEl);
          });
      })
      .catch((err: { message?: string }) => {
        if (disposed.current || !item) return;
        const errEl = document.createElement('div');
        errEl.className = 'read-later-action-error';
        errEl.textContent = err?.message || 'Failed to open link';
        item.appendChild(errEl);
      });
  };

  if (entries === null) {
    return <div className="read-later-loading">Loading…</div>;
  }

  return (
    <div onClick={onClick}>
      {showTabs ? <ReadLaterTabs filter={filter} /> : null}
      <div className={showTabs ? 'read-later-list-host' : undefined}>
        <ListBody
          entries={entries}
          showUnavailable={showUnavailable}
          message={UNAVAILABLE_MSG}
          filter={filter}
        />
      </div>
    </div>
  );
}

/** Tests only: createRoot into a container. Production uses ReadLaterDialog. */
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

    const chatBtn = target.closest('.read-later-open-in-chat') as HTMLButtonElement | null;
    if (chatBtn) {
      event.preventDefault();
      const { id } = chatBtn.dataset;
      const item = chatBtn.closest('.read-later-item');
      const entry = id ? lastSuccessfulEntries?.find((row) => row.id === id) : undefined;
      if (!entry) return;
      item?.querySelector('.read-later-action-error')?.remove();
      void openReadLaterInChat(entry, chatBtn).catch((err: { message?: string }) => {
        if (disposed || !item) return;
        const errEl = document.createElement('div');
        errEl.className = 'read-later-action-error';
        errEl.textContent = err?.message || 'Failed to open in chat';
        item.appendChild(errEl);
      });
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
