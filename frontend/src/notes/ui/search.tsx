import { useEffect, useRef, useState, type ReactNode } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { searchWorkbench } from '../../host/api.ts';
import { attachDigestTooltip } from './digest-tooltip.tsx';

const HIST_KEY = 'gs-history-wb';
const HIST_MAX = 10;

type WbHit = {
  title?: string;
  common_path?: string;
  layer?: string;
  topic?: string;
  body?: string;
  _formatted?: { body?: string };
};

type DropdownView =
  | { kind: 'hidden' }
  | { kind: 'history'; queries: string[] }
  | { kind: 'status'; text: ReactNode; color?: string }
  | { kind: 'hits'; hits: WbHit[] };

const closeListeners = new Set<() => void>();

function normalizeQuery(raw: unknown) {
  const trimmed = String(raw || '').trim();
  return trimmed.startsWith('#') ? trimmed.slice(1).trim() : trimmed;
}

function getHistory(): string[] {
  try {
    return JSON.parse(localStorage.getItem(HIST_KEY) || '') || [];
  } catch {
    return [];
  }
}

function addHistory(q: string) {
  const normalized = normalizeQuery(q);
  if (!normalized) return;
  const list = getHistory().filter((x) => x !== normalized);
  list.unshift(normalized);
  localStorage.setItem(HIST_KEY, JSON.stringify(list.slice(0, HIST_MAX)));
}

function removeHistory(q: string) {
  const list = getHistory().filter((x) => x !== q);
  localStorage.setItem(HIST_KEY, JSON.stringify(list));
}

function getSnippet(hit: WbHit) {
  const formatted = hit._formatted || {};
  const body = formatted.body || hit.body || '';
  const plain = body.replace(/^#{1,6}\s+/gm, '').replace(/[*_`]/g, '');
  return escKeepEm(plain.slice(0, 120).trim());
}

function escKeepEm(str: string) {
  return str
    .replace(/&/g, '&amp;')
    .replace(/<em>/g, '\x00EM\x00')
    .replace(/<\/em>/g, '\x00_EM\x00')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/\x00EM\x00/g, '<em>')
    .replace(/\x00_EM\x00/g, '</em>');
}

function hideLeftoverDropdown() {
  const dropdown = document.getElementById('gs-wb-dropdown');
  if (dropdown) dropdown.style.display = 'none';
}

export function closeWorkbenchSearch() {
  hideLeftoverDropdown();
  closeListeners.forEach((fn) => fn());
}

function GsStatus({ children, color }: { children: ReactNode; color?: string }) {
  return (
    <div className="gs-status" style={color ? { color } : undefined}>
      {children}
    </div>
  );
}

function GsHitRow({ hit, onOpen }: { hit: WbHit; onOpen: (hit: WbHit) => void }) {
  const rowRef = useRef<HTMLDivElement | null>(null);
  const cp = hit.common_path || '';

  useEffect(() => {
    const el = rowRef.current;
    if (!el) return;
    return attachDigestTooltip(el, cp);
  }, [cp]);

  return (
    <div
      ref={rowRef}
      className="gs-hit gs-hit-wb"
      data-common-path={cp}
      data-layer={hit.layer || ''}
      onClick={() => onOpen(hit)}
    >
      <div className="gs-hit-title">{hit.title || hit.common_path || ''}</div>
      <span className="gs-hit-repo">{hit.topic || ''}</span>
      <div className="gs-hit-snippet" dangerouslySetInnerHTML={{ __html: getSnippet(hit) }} />
    </div>
  );
}

export function WorkbenchSearchFields() {
  const [query, setQuery] = useState('');
  const [view, setView] = useState<DropdownView>({ kind: 'hidden' });
  const wrapRef = useRef<HTMLDivElement | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  function close() {
    setView({ kind: 'hidden' });
    hideLeftoverDropdown();
  }

  useEffect(() => {
    const onClose = () => {
      setView({ kind: 'hidden' });
      hideLeftoverDropdown();
    };
    closeListeners.add(onClose);
    return () => {
      closeListeners.delete(onClose);
    };
  }, []);

  useEffect(() => {
    const onDocClick = (e: MouseEvent) => {
      const wrap = wrapRef.current || document.getElementById('gs-wb-wrap');
      if (wrap && !wrap.contains(e.target as Node)) close();
    };
    document.addEventListener('click', onDocClick);
    return () => document.removeEventListener('click', onDocClick);
  }, []);

  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, []);

  async function runSearch(q: string) {
    flushSync(() => setView({ kind: 'status', text: 'Searching…' }));
    try {
      const data = await searchWorkbench(q, 8);
      if (data.error && data.error !== 'not_indexed') {
        flushSync(() => setView({ kind: 'status', text: 'Search error' }));
        return;
      }
      if (data.error === 'not_indexed') {
        flushSync(() => setView({ kind: 'status', text: 'Index not built yet.' }));
        return;
      }
      const hits = (data.hits || []) as WbHit[];
      if (hits.length === 0) flushSync(() => setView({ kind: 'status', text: 'No related results' }));
      else flushSync(() => setView({ kind: 'hits', hits }));
    } catch {
      flushSync(() => setView({ kind: 'status', text: 'Search error' }));
    }
  }

  function onQueryChange(raw: string) {
    setQuery(raw);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    const q = normalizeQuery(raw);
    if (!q) {
      const list = getHistory();
      setView(list.length ? { kind: 'history', queries: list } : { kind: 'hidden' });
      return;
    }
    debounceRef.current = setTimeout(() => void runSearch(q), 300);
  }

  function onFocus() {
    if (!normalizeQuery(query)) {
      const list = getHistory();
      if (list.length) setView({ kind: 'history', queries: list });
    }
  }

  function openHit(hit: WbHit) {
    const cp = hit.common_path;
    if (cp) {
      document.dispatchEvent(new CustomEvent('cta:open-entry', { detail: { common_path: cp } }));
    }
    addHistory(query);
    close();
  }

  const open = view.kind !== 'hidden';

  return (
    <>
      <input
        ref={inputRef}
        id="gs-wb-input"
        className="gs-search-input"
        type="text"
        placeholder="Search notes…"
        autoComplete="off"
        spellCheck={false}
        value={query}
        onChange={(e) => onQueryChange(e.target.value)}
        onInput={(e) => onQueryChange((e.target as HTMLInputElement).value)}
        onFocus={onFocus}
        onKeyDown={(e) => {
          if (e.key === 'Escape') {
            close();
            inputRef.current?.blur();
          }
        }}
      />
      <div
        id="gs-wb-dropdown"
        className="gs-search-dropdown"
        style={{ display: open ? 'block' : 'none' }}
      >
        {view.kind === 'status' ? <GsStatus color={view.color}>{view.text}</GsStatus> : null}
        {view.kind === 'hits'
          ? view.hits.map((hit, i) => (
              <GsHitRow key={`${hit.common_path || ''}-${hit.layer || ''}-${i}`} hit={hit} onOpen={openHit} />
            ))
          : null}
        {view.kind === 'history' ? (
          <div className="gs-hist-list">
            {view.queries.map((q) => (
              <div
                key={q}
                className="gs-hist-item"
                data-q={q}
                onClick={() => onQueryChange(q)}
              >
                <span className="gs-hist-label">{q}</span>
                <button
                  type="button"
                  className="gs-hist-remove"
                  data-q={q}
                  title="Delete"
                  onClick={(e) => {
                    e.stopPropagation();
                    removeHistory(q);
                    const list = getHistory();
                    setView(list.length ? { kind: 'history', queries: list } : { kind: 'hidden' });
                  }}
                >
                  ×
                </button>
              </div>
            ))}
          </div>
        ) : null}
      </div>
    </>
  );
}

export function WorkbenchSearch() {
  return (
    <div id="gs-wb-wrap" className="gs-search-wrap" hidden data-wb-search="react">
      <WorkbenchSearchFields />
    </div>
  );
}

let _initialized = false;
let _testRoot: Root | null = null;

/** Production: Shell already renders WorkbenchSearch. Tests: mount fields into leftover wrap. */
export function initWorkbenchSearch() {
  if (_initialized) return;
  const wrap = document.getElementById('gs-wb-wrap');
  if (!wrap) return;
  _initialized = true;
  if (wrap.dataset.wbSearch === 'react') return;
  _testRoot = createRoot(wrap);
  flushSync(() => {
    _testRoot!.render(<WorkbenchSearchFields />);
  });
  wrap.dataset.wbSearch = 'react';
}
