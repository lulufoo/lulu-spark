import { useEffect, useRef, useState, type KeyboardEvent } from 'react';
import { searchSpark } from '../../host/api.ts';
import {
  addSearchHistory,
  getSearchHistory,
  normalizeSearchQuery,
  removeSearchHistory,
} from './history.ts';
import { hitLocation, type SearchHit, type SearchNames, type SearchScope } from './hit-location.ts';

const RESULT_LIMIT = 8;
const DEBOUNCE_MS = 300;

type View =
  | { kind: 'history' }
  | { kind: 'status'; text: string }
  | { kind: 'hits'; hits: SearchHit[] };

function openEntry(id: string) {
  document.dispatchEvent(new CustomEvent('cta:open-entry', { detail: { id } }));
}

function SearchGlyph() {
  return (
    <svg className="search-modal-glyph" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <circle cx="11" cy="11" r="8" fill="none" stroke="currentColor" strokeWidth="2" />
      <path d="m21 21-4.3-4.3" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
    </svg>
  );
}

function SearchModalBody({
  scope,
  names,
  onOpenDoc,
  onClose,
}: {
  scope: SearchScope;
  names: SearchNames;
  onOpenDoc: (id: string) => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState('');
  const [history, setHistory] = useState<string[]>(() => getSearchHistory());
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
      const data = await searchSpark(
        q,
        RESULT_LIMIT,
        scope.kind === 'workspace' ? scope.workspaceId : '',
        { allWorkspaces: scope.kind === 'all-workspaces' },
      );
      if (request !== requestRef.current) return;
      if (data.error === 'not_indexed') {
        setView({ kind: 'status', text: 'Index not built yet.' });
      } else if (data.error) {
        setView({ kind: 'status', text: 'Search error' });
      } else {
        const hits = (data.hits || []) as SearchHit[];
        setView(hits.length ? { kind: 'hits', hits } : { kind: 'status', text: 'No related results' });
      }
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
      setHistory(getSearchHistory());
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

  function openHit(hit: SearchHit) {
    if (hit.id) onOpenDoc(hit.id);
    addSearchHistory(query);
    onClose();
  }

  const rowCount = view.kind === 'hits' ? view.hits.length : view.kind === 'history' ? history.length : 0;

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
      const hit = view.hits[active];
      if (hit) openHit(hit);
    } else if (view.kind === 'history') {
      const q = history[active];
      if (q) applyQuery(q, true);
    } else if (normalizeSearchQuery(query)) {
      // A status line (error, no results): search again.
      applyQuery(query, true);
    }
  }

  const listId = 'search-modal-list';
  const rowId = (i: number) => `search-modal-row-${i}`;

  return (
    <div
      className="search-modal-backdrop"
      data-role="search-modal"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="search-modal" role="dialog" aria-modal="true" aria-label="Search documents">
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
            aria-activedescendant={rowCount > 0 ? rowId(active) : undefined}
            placeholder={scope.kind === 'all-workspaces' ? 'Search all workspaces…' : 'Search this workspace…'}
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
        <div id={listId} className="search-modal-list" role="listbox" ref={listRef}>
          {view.kind === 'status' ? <div className="search-modal-status">{view.text}</div> : null}
          {view.kind === 'hits'
            ? view.hits.map((hit, i) => {
                const location = hitLocation(hit, scope, names);
                return (
                  <div
                    key={`${hit.id || ''}-${i}`}
                    id={rowId(i)}
                    className="search-modal-row"
                    role="option"
                    aria-selected={i === active}
                    data-role="search-hit"
                    data-note-id={hit.id || ''}
                    data-active={i === active ? 'true' : undefined}
                    onMouseEnter={() => setActive(i)}
                    onClick={() => openHit(hit)}
                  >
                    <span className="search-modal-row-title">{hit.title || hit.id || ''}</span>
                    {location ? <span className="search-modal-row-sub">{location}</span> : null}
                  </div>
                );
              })
            : null}
          {view.kind === 'history' && history.length === 0 ? (
            <div className="search-modal-status">Type to search documents.</div>
          ) : null}
          {view.kind === 'history' && history.length > 0 ? (
            <>
              <div className="search-modal-section">Recent searches</div>
              {history.map((q, i) => (
                <div
                  key={q}
                  id={rowId(i)}
                  className="search-modal-row search-modal-row--history"
                  role="option"
                  aria-selected={i === active}
                  data-role="search-history-item"
                  data-q={q}
                  data-active={i === active ? 'true' : undefined}
                  onMouseEnter={() => setActive(i)}
                  onClick={() => applyQuery(q, true)}
                >
                  <span className="search-modal-row-title">{q}</span>
                  <button
                    type="button"
                    className="search-modal-history-remove"
                    data-role="search-history-remove"
                    data-q={q}
                    data-tip="Delete"
                    aria-label={`Delete “${q}” from recent searches`}
                    onClick={(e) => {
                      e.stopPropagation();
                      removeSearchHistory(q);
                      const next = getSearchHistory();
                      setHistory(next);
                      setActive((a) => Math.min(a, Math.max(0, next.length - 1)));
                    }}
                  >
                    ×
                  </button>
                </div>
              ))}
            </>
          ) : null}
        </div>
      </div>
    </div>
  );
}

/**
 * The document search modal shared by Home and the notes page. Controlled: the page decides when it
 * is `open` and what `scope` it covers. Documents only; an empty query lists the recent searches.
 * `onOpenDoc` defaults to the shared `cta:open-entry` event; Home passes its own opener because
 * the notes index may not be loaded there.
 */
export function SearchModal({
  open,
  scope,
  names,
  onOpenDoc = openEntry,
  onClose,
}: {
  open: boolean;
  scope: SearchScope;
  names: SearchNames;
  onOpenDoc?: (id: string) => void;
  onClose: () => void;
}) {
  if (!open) return null;
  return <SearchModalBody scope={scope} names={names} onOpenDoc={onOpenDoc} onClose={onClose} />;
}
