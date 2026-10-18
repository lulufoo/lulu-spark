import { useEffect, useRef, useState, type KeyboardEvent } from 'react';
import {
  addSearchHistory,
  getSearchHistory,
  normalizeSearchQuery,
  removeSearchHistory,
} from './history.ts';
import type { SearchNames, SearchScope } from './hit-location.ts';
import {
  getHistoryKey,
  getModeTexts,
  searchByMode,
  type ResultRow,
  type SearchMode,
} from './modal-mode.ts';
import { getRowId, SearchResultList, type View } from './search-result-list.tsx';

const RESULT_LIMIT = 8;
const DEBOUNCE_MS = 300;

function openEntry(id: string) {
  document.dispatchEvent(new CustomEvent('cta:open-entry', { detail: { id } }));
}

export function SearchGlyph({ className = 'search-modal-glyph' }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <circle cx="11" cy="11" r="8" fill="none" stroke="currentColor" strokeWidth="2" />
      <path d="m21 21-4.3-4.3" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
    </svg>
  );
}

function SearchModalBody({
  mode,
  scope,
  names,
  onOpenDoc,
  onOpenChat,
  onClose,
}: {
  mode: SearchMode;
  scope: SearchScope;
  names: SearchNames;
  onOpenDoc: (id: string) => void;
  onOpenChat: (id: string) => void;
  onClose: () => void;
}) {
  const historyKey = getHistoryKey(mode);
  const texts = getModeTexts(mode, scope);
  const [query, setQuery] = useState('');
  const [history, setHistory] = useState<string[]>(() => getSearchHistory(historyKey));
  const [view, setView] = useState<View>({ kind: 'history' });
  const [active, setActive] = useState(0);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const listRef = useRef<HTMLDivElement | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Only the newest request may paint; an older answer that arrives late is dropped.
  const requestRef = useRef(0);

  useEffect(() => {
    inputRef.current?.focus();
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = null;
      requestRef.current += 1;
    };
  }, []);

  useEffect(() => {
    const row = listRef.current?.querySelector('[data-active="true"]');
    if (row && typeof (row as HTMLElement).scrollIntoView === 'function') {
      (row as HTMLElement).scrollIntoView({ block: 'nearest' });
    }
  }, [active, view]);

  async function runSearch(q: string) {
    const request = ++requestRef.current;
    setView({ kind: 'status', text: 'Searching…' });
    try {
      const result = await searchByMode(mode, q, RESULT_LIMIT, scope, names);
      if (request !== requestRef.current) return;
      if (result.status) setView({ kind: 'status', text: result.status });
      else if (result.rows.length) setView({ kind: 'hits', rows: result.rows });
      else setView({ kind: 'status', text: texts.noResults });
    } catch {
      if (request === requestRef.current) setView({ kind: 'status', text: 'Search error' });
    }
  }

  function clearPending() {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = null;
  }

  /** Set the query; search now (`immediate`) or after the typing pause. */
  function applyQuery(raw: string, immediate = false) {
    setQuery(raw);
    setActive(0);
    clearPending();
    const q = normalizeSearchQuery(raw);
    if (!q) {
      requestRef.current += 1;
      setHistory(getSearchHistory(historyKey));
      setView({ kind: 'history' });
      return;
    }
    if (immediate) {
      void runSearch(q);
    } else {
      debounceRef.current = setTimeout(() => {
        debounceRef.current = null;
        void runSearch(q);
      }, DEBOUNCE_MS);
    }
  }

  function openRow(row: ResultRow) {
    if (row.id) (mode === 'chats' ? onOpenChat : onOpenDoc)(row.id);
    addSearchHistory(query, historyKey);
    onClose();
  }

  const rowCount = view.kind === 'hits' ? view.rows.length : view.kind === 'history' ? history.length : 0;

  function onKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
      return;
    }
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      if (rowCount === 0) return;
      setActive((i) => Math.min(rowCount - 1, Math.max(0, i + (e.key === 'ArrowDown' ? 1 : -1))));
      return;
    }
    // Enter that confirms an IME composition must not act on the list.
    if (e.key !== 'Enter' || e.nativeEvent.isComposing) return;
    e.preventDefault();
    if (debounceRef.current) {
      // The query changed and its search is still waiting for the typing pause: search now.
      applyQuery(query, true);
    } else if (view.kind === 'hits') {
      const row = view.rows[active];
      if (row) openRow(row);
    } else if (view.kind === 'history') {
      const q = history[active];
      if (q) applyQuery(q, true);
    } else if (normalizeSearchQuery(query)) {
      // A status line (error, no results): search again.
      applyQuery(query, true);
    }
  }

  const listId = 'search-modal-list';

  return (
    <div
      className="search-modal-backdrop"
      data-role="search-modal"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="search-modal" role="dialog" aria-modal="true" aria-label={texts.label}>
        <div className="search-modal-head">
          <SearchGlyph />
          <input
            ref={inputRef}
            className="search-modal-input"
            data-role="search-modal-input"
            type="text"
            role="combobox"
            aria-expanded="true"
            aria-controls={listId}
            aria-activedescendant={rowCount > 0 ? getRowId(active) : undefined}
            placeholder={texts.placeholder}
            autoComplete="off"
            spellCheck={false}
            value={query}
            onChange={(e) => applyQuery(e.target.value)}
            onKeyDown={onKeyDown}
          />
          <button type="button" className="search-modal-esc" data-role="search-modal-close" onClick={onClose}>
            esc
          </button>
        </div>
        <SearchResultList
          listId={listId}
          listRef={listRef}
          mode={mode}
          view={view}
          history={history}
          emptyHistoryText={texts.emptyHistory}
          active={active}
          onActive={setActive}
          onOpenRow={openRow}
          onPickHistory={(q) => applyQuery(q, true)}
          onRemoveHistory={(q) => {
            removeSearchHistory(q, historyKey);
            const next = getSearchHistory(historyKey);
            setHistory(next);
            setActive((a) => Math.min(a, Math.max(0, next.length - 1)));
          }}
        />
      </div>
    </div>
  );
}

/**
 * The search modal shared by Home and the notes page. Controlled: the page decides when it is
 * `open`, what `scope` it covers and which `mode` it searches: documents (default) or chats by
 * title. An empty query lists the recent searches of that mode. `onOpenDoc` defaults to the shared
 * `cta:open-entry` event; Home passes its own opener because the notes index may not be loaded
 * there. `onOpenChat` is called with the chat id when a chat row is chosen.
 */
export function SearchModal({
  open,
  mode = 'docs',
  scope,
  names,
  onOpenDoc = openEntry,
  onOpenChat = () => {},
  onClose,
}: {
  open: boolean;
  mode?: SearchMode;
  scope: SearchScope;
  names: SearchNames;
  onOpenDoc?: (id: string) => void;
  onOpenChat?: (id: string) => void;
  onClose: () => void;
}) {
  if (!open) return null;
  // `key` restarts the body when the mode changes, so one mode's query never shows in the other.
  return (
    <SearchModalBody
      key={mode}
      mode={mode}
      scope={scope}
      names={names}
      onOpenDoc={onOpenDoc}
      onOpenChat={onOpenChat}
      onClose={onClose}
    />
  );
}
