import { useCallback, useEffect, useRef, useState } from 'react';
import { flushSync } from 'react-dom';
import { createRoot, type Root } from 'react-dom/client';
import { bindFocusRefresh, markEntryRead, openExternalUrl } from '../commands/list.ts';
import { loadAssistantEntries, selectTop3Latest } from '../commands/assistant.ts';
import type { ReadLaterEntry } from '../state/types.ts';

export { bindFocusRefresh };
export { loadAssistantEntries, selectTop3Latest };

const UNAVAILABLE_MSG = 'List temporarily unavailable. Please try again later.';

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

function ManageLink({ showManage, onManage }: { showManage: boolean; onManage: () => void }) {
  if (!showManage) return null;
  return (
    <button type="button" className="read-later-assistant-manage-link" onClick={onManage}>
      View all read-later →
    </button>
  );
}

function Panel({
  entries,
  selectedIndex,
  onSelect,
  onOpen,
  onCycle,
}: {
  entries: ReadLaterEntry[];
  selectedIndex: number;
  onSelect: (index: number) => void;
  onOpen: (entry: ReadLaterEntry) => void;
  onCycle: () => void;
}) {
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
                onClick={() => onSelect(index)}
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
            onClick={(event) => {
              event.preventDefault();
              onOpen(current);
            }}
          >
            <h3 className="read-later-assistant-current-title">{current.title || current.url}</h3>
            <p className="read-later-assistant-current-url" title={current.url}>
              {current.url}
            </p>
          </a>
          <p className="read-later-assistant-current-saved-at">{formatSavedAt(current.saved_at)}</p>
        </div>
        {showCycle ? (
          <button type="button" className="read-later-assistant-cycle" aria-label="Next" onClick={onCycle}>
            ›
          </button>
        ) : null}
      </article>
    </div>
  );
}

export function ReadLaterAssistant({
  autoLoad = true,
  navigate,
  openReadLater,
  handleRef,
}: {
  autoLoad?: boolean;
  navigate?: (hash: string) => void;
  openReadLater?: () => void;
  handleRef?: { current: { refresh: () => Promise<void> } };
}) {
  const [loading, setLoading] = useState(false);
  const [entries, setEntries] = useState<ReadLaterEntry[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [showUnavailable, setShowUnavailable] = useState(false);
  const [errorOnly, setErrorOnly] = useState(false);
  const refreshPromise = useRef<Promise<void> | null>(null);
  const snapshotRef = useRef<ReadLaterEntry[] | null>(null);
  const disposedRef = useRef(false);
  const showManage = typeof openReadLater === 'function' || typeof navigate === 'function';

  const refresh = useCallback(async () => {
    if (refreshPromise.current) return refreshPromise.current;
    refreshPromise.current = (async () => {
      if (snapshotRef.current === null) setLoading(true);
      try {
        const next = selectTop3Latest((await loadAssistantEntries()) as ReadLaterEntry[]);
        snapshotRef.current = next;
        setEntries(next);
        setSelectedIndex(0);
        setShowUnavailable(false);
        setErrorOnly(false);
      } catch {
        if (snapshotRef.current !== null) {
          setEntries(snapshotRef.current);
          setShowUnavailable(true);
          setErrorOnly(false);
        } else {
          setEntries([]);
          setSelectedIndex(0);
          setErrorOnly(true);
        }
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
    disposedRef.current = false;
    const dispose = bindFocusRefresh(refresh);
    if (autoLoad) void refresh();
    return () => {
      disposedRef.current = true;
      dispose();
    };
  }, [autoLoad, refresh]);

  function onManage() {
    if (typeof openReadLater === 'function') openReadLater();
    else if (typeof navigate === 'function') navigate('#/read-later');
  }

  function onOpen(entry: ReadLaterEntry) {
    const url = entry.url;
    const id = entry.id;
    if (!url) return;
    void openExternalUrl(url)
      .then(() => {
        if (disposedRef.current || !id || entry.read) return;
        return markEntryRead(id);
      })
      .then(() => {
        if (disposedRef.current || !id || entry.read) return;
        setEntries((prev) => {
          const next = prev.map((item) => (item.id === id ? { ...item, read: true } : item));
          snapshotRef.current = next;
          return next;
        });
      })
      .catch((err: unknown) => {
        console.error('[read-later-assistant] open link or mark read failed', err);
      });
  }

  const manage = <ManageLink showManage={showManage} onManage={onManage} />;

  if (loading) {
    return <div className="read-later-assistant-loading">Loading…</div>;
  }
  if (errorOnly) {
    return (
      <div>
        <ErrorEmpty />
        {manage}
      </div>
    );
  }
  if (!entries.length) {
    return (
      <div>
        {showUnavailable ? <UnavailableBanner /> : null}
        {showUnavailable ? <ErrorEmpty /> : <EmptyState />}
        {manage}
      </div>
    );
  }
  return (
    <div>
      {showUnavailable ? <UnavailableBanner /> : null}
      <Panel
        entries={entries}
        selectedIndex={selectedIndex}
        onSelect={(index) => {
          flushSync(() => setSelectedIndex(index));
        }}
        onOpen={onOpen}
        onCycle={() => {
          flushSync(() => {
            setSelectedIndex((index) => (entries.length ? (index + 1) % entries.length : 0));
          });
        }}
      />
      {manage}
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
  const handleRef = { current: { refresh: () => Promise.resolve() } };
  const reactRoot: Root = createRoot(rootEl);
  flushSync(() => {
    reactRoot.render(<ReadLaterAssistant {...opts} handleRef={handleRef} />);
  });
  return {
    dispose() {
      flushSync(() => {
        reactRoot.unmount();
      });
      rootEl.innerHTML = '';
    },
    refresh() {
      return handleRef.current.refresh();
    },
  };
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
