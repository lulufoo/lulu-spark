import type { MouseEvent } from 'react';
import { flushSync } from 'react-dom';
import { createRoot, type Root } from 'react-dom/client';
import {
  bindFocusRefresh,
  loadReadLaterEntries,
  markEntryRead,
  openExternalUrl,
} from './list.tsx';

export { bindFocusRefresh };

const UNAVAILABLE_MSG = 'List temporarily unavailable. Please try again later.';

type AssistantEntry = {
  id: string;
  title?: string;
  url: string;
  saved_at?: string;
  read?: boolean;
};

export async function loadAssistantEntries() {
  return loadReadLaterEntries();
}

export function selectTop3Latest(entries: AssistantEntry[]) {
  return [...entries]
    .sort((a, b) => {
      const aTime = Date.parse(a.saved_at ?? '') || 0;
      const bTime = Date.parse(b.saved_at ?? '') || 0;
      return bTime - aTime;
    })
    .slice(0, 3);
}

function formatSavedAt(savedAt: string | undefined) {
  if (!savedAt) return '';
  const date = new Date(savedAt);
  if (Number.isNaN(date.getTime())) return savedAt;
  return date.toLocaleString('en', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

function entryReadClass(baseClass: string, read: boolean | undefined) {
  const stateClass = read
    ? 'read-later-assistant-current--read'
    : 'read-later-assistant-current--unread';
  return `${baseClass} ${stateClass}`;
}

function EmptyState() {
  return (
    <div className="read-later-assistant-empty read-later-assistant-state">
      <p className="read-later-assistant-state-title">No items to read later</p>
      <p className="read-later-assistant-state-detail">
        After saving with the Chrome extension, latest items appear here
      </p>
    </div>
  );
}

function UnavailableBanner({ message = UNAVAILABLE_MSG }: { message?: string }) {
  return (
    <div className="read-later-assistant-unavailable read-later-assistant-banner">{message}</div>
  );
}

function ErrorEmpty({ message = UNAVAILABLE_MSG }: { message?: string }) {
  return (
    <div className="read-later-assistant-empty read-later-assistant-state read-later-assistant-state--error">
      <p className="read-later-assistant-state-title">Temporarily unavailable</p>
      <p className="read-later-assistant-state-detail">{message}</p>
    </div>
  );
}

function ManageLink({ showManage }: { showManage: boolean }) {
  if (!showManage) return null;
  return (
    <button type="button" className="read-later-assistant-manage-link">
      View all read-later →
    </button>
  );
}

function Panel({ entries, selectedIndex }: { entries: AssistantEntry[]; selectedIndex: number }) {
  const current = entries[selectedIndex];
  const showCycle = entries.length > 1;
  const currentClass = entryReadClass(
    showCycle
      ? 'read-later-assistant-current read-later-assistant-current--has-cycle'
      : 'read-later-assistant-current',
    current.read,
  );
  return (
    <div className="read-later-assistant-panel">
      {showCycle ? (
        <div className="read-later-assistant-picker" role="tablist" aria-label="Recent items">
          {entries.map((entry, index) => {
            const activeClass =
              index === selectedIndex ? ' read-later-assistant-picker-item--active' : '';
            const readClass = entry.read
              ? ' read-later-assistant-picker-item--read'
              : ' read-later-assistant-picker-item--unread';
            const label = entry.title || entry.url;
            return (
              <button
                key={entry.id}
                type="button"
                className={`read-later-assistant-picker-item${readClass}${activeClass}`}
                data-entry-id={entry.id}
                data-index={index}
                title={label}
              >
                <span className="read-later-assistant-picker-rank">{index + 1}</span>
                <span className="read-later-assistant-picker-label">{label}</span>
              </button>
            );
          })}
        </div>
      ) : null}
      <article className={currentClass}>
        <div className="read-later-assistant-current-main">
          <a
            className="read-later-assistant-entry-link"
            href={current.url}
            data-url={current.url}
            data-entry-id={current.id}
          >
            <h3 className="read-later-assistant-current-title">{current.title || current.url}</h3>
            <p className="read-later-assistant-current-url" title={current.url}>
              {current.url}
            </p>
          </a>
          <p className="read-later-assistant-current-saved-at">{formatSavedAt(current.saved_at)}</p>
        </div>
        {showCycle ? (
          <button type="button" className="read-later-assistant-cycle" aria-label="Next">
            ›
          </button>
        ) : null}
      </article>
    </div>
  );
}

export function mountReadLaterAssistant(
  rootEl: HTMLElement,
  opts: {
    autoLoad?: boolean;
    navigate?: (hash: string) => void;
    openReadLater?: () => void;
  } = {},
) {
  const { autoLoad = true, navigate, openReadLater } = opts;
  let disposed = false;
  let top3Entries: AssistantEntry[] = [];
  let selectedIndex = 0;
  let lastSuccessfulTop3: AssistantEntry[] | null = null;
  let refreshPromise: Promise<void> | null = null;
  let reactRoot: Root | null = createRoot(rootEl);
  let showUnavailable = false;
  let errorOnly = false;

  const showManage = typeof openReadLater === 'function' || typeof navigate === 'function';

  function paint(loading = false) {
    if (!reactRoot || disposed) return;
    flushSync(() => {
      if (loading) {
        reactRoot!.render(<div className="read-later-assistant-loading">Loading…</div>);
        return;
      }
      if (errorOnly) {
        reactRoot!.render(
          <div onClick={onClick}>
            <ErrorEmpty />
            <ManageLink showManage={showManage} />
          </div>,
        );
        return;
      }
      if (!top3Entries.length) {
        reactRoot!.render(
          <div onClick={onClick}>
            {showUnavailable ? <UnavailableBanner /> : null}
            {showUnavailable ? <ErrorEmpty /> : <EmptyState />}
            <ManageLink showManage={showManage} />
          </div>,
        );
        return;
      }
      reactRoot!.render(
        <div onClick={onClick}>
          {showUnavailable ? <UnavailableBanner /> : null}
          <Panel entries={top3Entries} selectedIndex={selectedIndex} />
          <ManageLink showManage={showManage} />
        </div>,
      );
    });
  }

  async function refreshAssistantTop3() {
    if (refreshPromise) {
      return refreshPromise;
    }

    refreshPromise = (async () => {
      const hasSnapshot = lastSuccessfulTop3 !== null;
      if (!hasSnapshot) {
        paint(true);
      }

      try {
        const entries = (await loadAssistantEntries()) as AssistantEntry[];
        if (disposed) return;
        top3Entries = selectTop3Latest(entries);
        lastSuccessfulTop3 = top3Entries;
        selectedIndex = 0;
        showUnavailable = false;
        errorOnly = false;
        paint();
      } catch {
        if (disposed) return;
        if (lastSuccessfulTop3 !== null) {
          top3Entries = lastSuccessfulTop3;
          showUnavailable = true;
          errorOnly = false;
          paint();
        } else {
          top3Entries = [];
          selectedIndex = 0;
          errorOnly = true;
          paint();
        }
      }
    })().finally(() => {
      refreshPromise = null;
    });

    return refreshPromise;
  }

  const onClick = (event: MouseEvent<HTMLElement>) => {
    const target = event.target as HTMLElement;
    if (target.closest('.read-later-assistant-manage-link')) {
      if (typeof openReadLater === 'function') openReadLater();
      else if (typeof navigate === 'function') navigate('#/read-later');
      return;
    }

    const pickerItem = target.closest('.read-later-assistant-picker-item') as HTMLElement | null;
    if (pickerItem) {
      const index = Number(pickerItem.dataset.index);
      if (!Number.isNaN(index) && index >= 0 && index < top3Entries.length) {
        selectedIndex = index;
        paint();
      }
      return;
    }

    const entryLink = target.closest('.read-later-assistant-entry-link') as HTMLAnchorElement | null;
    if (entryLink) {
      event.preventDefault();
      const url = entryLink.dataset.url || entryLink.getAttribute('href');
      const id = entryLink.dataset.entryId;
      if (!url) return;
      const current = top3Entries[selectedIndex];
      void openExternalUrl(url)
        .then(() => {
          if (disposed || !id || current?.read) return;
          return markEntryRead(id);
        })
        .then(() => {
          if (disposed || !id || current?.read) return;
          top3Entries = top3Entries.map((entry) =>
            entry.id === id ? { ...entry, read: true } : entry,
          );
          lastSuccessfulTop3 = top3Entries;
          paint();
        })
        .catch((err: unknown) => {
          console.error('[read-later-assistant] open link or mark read failed', err);
        });
      return;
    }

    if (target.closest('.read-later-assistant-cycle')) {
      if (!top3Entries.length) return;
      selectedIndex = (selectedIndex + 1) % top3Entries.length;
      paint();
    }
  };

  const disposeFocusRefresh = bindFocusRefresh(refreshAssistantTop3);
  if (autoLoad) {
    void refreshAssistantTop3();
  }

  function dispose() {
    disposed = true;
    disposeFocusRefresh();
    flushSync(() => {
      reactRoot?.unmount();
    });
    reactRoot = null;
    rootEl.innerHTML = '';
  }

  return { dispose, refresh: refreshAssistantTop3 };
}

/** A11y / brand label for the Read Later content region (shell owns overlay title). */
export const READ_LATER_CONTENT_LABEL = 'Open Read Later assistant';

export function createReadLaterContentAdapter() {
  return {
    mount(
      slotEl: HTMLElement,
      ctx: { host?: { navigate?: (hash: string) => void; openReadLater?: () => void } } = {},
    ) {
      const host = ctx.host ?? {};
      slotEl.setAttribute('aria-label', READ_LATER_CONTENT_LABEL);
      const panel = mountReadLaterAssistant(slotEl, {
        autoLoad: true,
        navigate: typeof host.navigate === 'function' ? host.navigate : undefined,
        openReadLater: typeof host.openReadLater === 'function' ? host.openReadLater : undefined,
      });
      return {
        unmount() {
          panel.dispose();
          slotEl.removeAttribute('aria-label');
        },
      };
    },
  };
}
