import { useEffect, useRef, useState, type ReactNode } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { searchKnowledge, reindexKnowledge, getReindexStatus } from '../../host/api.ts';

const HIST_KEY = 'gs-history-kb';
const HIST_MAX = 10;

type KbHit = {
  title?: string;
  path?: string;
  repo?: string;
  url?: string;
  body?: string;
  _formatted?: { body?: string };
};

type DropdownView =
  | { kind: 'hidden' }
  | { kind: 'history'; queries: string[] }
  | { kind: 'status'; text: ReactNode; color?: string }
  | { kind: 'hits'; hits: KbHit[] };

const closeListeners = new Set<() => void>();

function normalizeQuery(raw: unknown) {
  return String(raw || '').trim();
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

function getSnippet(hit: KbHit) {
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
  const dropdown = document.getElementById('gs-kb-dropdown');
  if (dropdown) dropdown.style.display = 'none';
}

export function closeCorpusSearch() {
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

export function CorpusSearchFields() {
  const [query, setQuery] = useState('');
  const [focused, setFocused] = useState(false);
  const [view, setView] = useState<DropdownView>({ kind: 'hidden' });
  const [rebuilding, setRebuilding] = useState(false);
  const [rebuildTitle, setRebuildTitle] = useState('Rebuild knowledge index');
  const inputRef = useRef<HTMLInputElement | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

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
      const wrap = document.getElementById('gs-kb-wrap');
      if (wrap && !wrap.contains(e.target as Node)) close();
    };
    document.addEventListener('click', onDocClick);
    return () => document.removeEventListener('click', onDocClick);
  }, []);

  useEffect(() => {
    const input = inputRef.current;
    if (!input) return undefined;
    const onFocus = () => flushSync(() => setFocused(true));
    const onBlur = () => {
      setTimeout(() => flushSync(() => setFocused(false)), 200);
    };
    input.addEventListener('focus', onFocus);
    input.addEventListener('blur', onBlur);
    return () => {
      input.removeEventListener('focus', onFocus);
      input.removeEventListener('blur', onBlur);
    };
  }, []);

  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
      if (pollRef.current) clearInterval(pollRef.current);
    };
  }, []);

  async function runSearch(q: string) {
    flushSync(() => setView({ kind: 'status', text: 'Searching…' }));
    try {
      const data = (await searchKnowledge(q, 8)) as { error?: string; hits?: KbHit[] };
      if (data.error === 'unavailable') {
        flushSync(() =>
          setView({
            kind: 'status',
            text: (
              <>
                Meilisearch is not running; search unavailable
                <br />
                <span style={{ fontSize: 10, opacity: 0.7 }}>
                  Start external Meilisearch first (default localhost:7700)
                </span>
              </>
            ),
          }),
        );
        return;
      }
      if (data.error === 'not_indexed') {
        flushSync(() => setView({ kind: 'status', text: 'Index not built yet. Click ↺ to rebuild.' }));
        return;
      }
      const hits = data.hits || [];
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

  function openHit(hit: KbHit) {
    const repo = hit.repo;
    const path = hit.path;
    if (repo && path) {
      document.dispatchEvent(
        new CustomEvent('cta:open-kb-doc', {
          detail: { repo, path, url: hit.url, title: hit.title || path },
        }),
      );
    }
    addHistory(query);
    close();
  }

  async function startRebuild() {
    setRebuilding(true);
    try {
      const res = (await reindexKnowledge()) as { error?: string };
      if (res.error) {
        stopRebuild(true, res.error);
        return;
      }
    } catch {
      stopRebuild(true, 'Request failed');
      return;
    }
    pollRef.current = setInterval(() => void pollRebuild(), 2000);
  }

  async function pollRebuild() {
    try {
      const res = (await getReindexStatus()) as { status?: string; log?: string };
      if (res.status === 'done') stopRebuild(false, res.log);
      else if (res.status === 'error') stopRebuild(true, res.log || 'Rebuild failed');
    } catch {
      /* keep polling */
    }
  }

  function stopRebuild(isError: boolean, msg?: string) {
    if (pollRef.current) clearInterval(pollRef.current);
    pollRef.current = null;
    setRebuilding(false);
    setRebuildTitle(isError ? `Rebuild failed: ${msg}` : 'Rebuild knowledge index');
    const color = isError ? '#cf222e' : '#1a7f37';
    flushSync(() =>
      setView({
        kind: 'status',
        color,
        text: `${isError ? '❌ Rebuild failed: ' : '✅ Rebuild finished: '}${msg || (isError ? 'Unknown error' : '')}`,
      }),
    );
  }

  const open = view.kind !== 'hidden';
  const showRebuild = focused || rebuilding;

  return (
    <>
      <input
        ref={inputRef}
        id="gs-kb-input"
        className="gs-search-input"
        type="text"
        placeholder="Search knowledge…"
        autoComplete="off"
        spellCheck={false}
        value={query}
        onChange={(e) => onQueryChange(e.target.value)}
        onInput={(e) => onQueryChange((e.target as HTMLInputElement).value)}
        onFocus={() => {
          setFocused(true);
          if (!normalizeQuery(query)) {
            const list = getHistory();
            if (list.length) setView({ kind: 'history', queries: list });
          }
        }}
        onBlur={() => setTimeout(() => flushSync(() => setFocused(false)), 200)}
        onKeyDown={(e) => {
          if (e.key === 'Escape') {
            close();
            inputRef.current?.blur();
          }
        }}
      />
      <button
        id="gs-kb-rebuild-btn"
        type="button"
        className={`gs-rebuild-btn${rebuilding ? ' syncing' : ''}`}
        style={{ display: showRebuild ? 'inline-flex' : 'none' }}
        title={rebuildTitle}
        disabled={rebuilding}
        onClick={() => void startRebuild()}
      >
        ↺
      </button>
      <div id="gs-kb-dropdown" className="gs-search-dropdown" style={{ display: open ? 'block' : 'none' }}>
        {view.kind === 'status' ? <GsStatus color={view.color}>{view.text}</GsStatus> : null}
        {view.kind === 'hits'
          ? view.hits.map((hit, i) => (
              <div
                key={`${hit.repo || ''}-${hit.path || ''}-${i}`}
                className="gs-hit gs-hit-kb"
                data-repo={hit.repo || ''}
                data-path={hit.path || ''}
                data-url={hit.url || ''}
                data-title={hit.title || hit.path || ''}
                onClick={() => openHit(hit)}
              >
                <div className="gs-hit-title">{hit.title || hit.path || ''}</div>
                <span className="gs-hit-repo">{(hit.repo || '').split('/').pop()}</span>
                <div className="gs-hit-snippet" dangerouslySetInnerHTML={{ __html: getSnippet(hit) }} />
              </div>
            ))
          : null}
        {view.kind === 'history' ? (
          <div className="gs-hist-list">
            {view.queries.map((q) => (
              <div key={q} className="gs-hist-item" data-q={q} onClick={() => onQueryChange(q)}>
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

export function CorpusSearch() {
  return (
    <div id="gs-kb-wrap" className="gs-search-wrap" hidden data-kb-search="react">
      <CorpusSearchFields />
    </div>
  );
}

let _initialized = false;
let _testRoot: Root | null = null;

export function initCorpusSearch() {
  if (_initialized) return;
  const wrap = document.getElementById('gs-kb-wrap');
  if (!wrap) return;
  _initialized = true;
  if (wrap.dataset.kbSearch === 'react') return;
  _testRoot = createRoot(wrap);
  flushSync(() => {
    _testRoot!.render(<CorpusSearchFields />);
  });
  wrap.dataset.kbSearch = 'react';
}
