import { useEffect, useRef, useState, type ReactNode } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { searchKnowledge } from '../../host/api.ts';
import { createModuleStore } from '../../shared/module-store.ts';

type KbHit = {
  title?: string;
  path?: string;
  repo?: string;
  url?: string;
  body?: string;
  _formatted?: { body?: string };
};

type ResultsView =
  | { kind: 'status'; text: ReactNode }
  | { kind: 'hits'; hits: KbHit[] };

const triggerStore = createModuleStore({ query: '', token: 0 });
let liveTrigger: ((q: string) => void) | null = null;

function buildQuery(entry: { title?: string; common_path?: string }) {
  const parts: string[] = [];
  if (entry.title && entry.title.trim()) parts.push(entry.title.trim());
  const cp = entry.common_path || '';
  const filename = cp.split('/').pop() || '';
  const slug = filename.replace(/\.md$/, '').replace(/^\d{8,14}-/, '');
  if (slug && slug !== entry.title) parts.push(slug);
  return parts.join(' ');
}

function getSnippet(hit: KbHit) {
  const formatted = hit._formatted || {};
  const body = formatted.body || hit.body || '';
  const plain = body.replace(/^#{1,6}\s+/gm, '').replace(/[*_`]/g, '');
  return escKeepEm(plain.slice(0, 160).trim());
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

export function KnowledgeSearchPanel({
  entry,
}: {
  entry?: { title?: string; common_path?: string } | null;
}) {
  const [query, setQuery] = useState('');
  const [collapsed, setCollapsed] = useState(() =>
    Boolean(document.getElementById('knowledge-panel')?.classList.contains('ks-collapsed')),
  );
  const [results, setResults] = useState<ResultsView>({ kind: 'status', text: '' });
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const searchRef = useRef<(q: string) => Promise<void>>(async () => {});
  const trigger = triggerStore.getSnapshot();

  useEffect(() => {
    liveTrigger = (q: string) => {
      flushSync(() => setQuery(q));
      void searchRef.current(q);
    };
    return () => {
      liveTrigger = null;
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, []);

  useEffect(() => {
    if (entry) {
      const q = buildQuery(entry);
      setQuery(q);
      void runSearch(q);
    }
  }, [entry?.common_path, entry?.title]);

  useEffect(() => {
    if (!trigger.query && !trigger.token) return;
    setQuery(trigger.query);
    void runSearch(trigger.query);
  }, [trigger.token, trigger.query]);

  async function runSearch(q: string) {
    if (!q) {
      flushSync(() => setResults({ kind: 'status', text: 'Enter keywords to search' }));
      return;
    }
    flushSync(() => setResults({ kind: 'status', text: 'Searching…' }));
    try {
      const data = (await searchKnowledge(q, 10)) as { error?: string; hits?: KbHit[] };
      if (data.error && data.error !== 'not_indexed') {
        flushSync(() => setResults({ kind: 'status', text: 'Search error' }));
        return;
      }
      if (data.error === 'not_indexed') {
        flushSync(() =>
          setResults({
            kind: 'status',
            text: (
              <>
                Knowledge index not built
                <br />
                <span style={{ fontSize: 10, color: '#aaa' }}>Use ↺ Index in the header</span>
              </>
            ),
          }),
        );
        return;
      }
      const hits = data.hits || [];
      if (hits.length === 0) {
        flushSync(() =>
          setResults({
            kind: 'status',
            text: (
              <>
                No related knowledge
                <br />
                <span style={{ fontSize: 10, color: '#aaa' }}>Try another keyword?</span>
              </>
            ),
          }),
        );
      } else {
        flushSync(() => setResults({ kind: 'hits', hits }));
      }
    } catch {
      flushSync(() => setResults({ kind: 'status', text: 'Search error' }));
    }
  }
  searchRef.current = runSearch;

  function onQueryChange(raw: string) {
    setQuery(raw);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => void runSearch(raw.trim()), 300);
  }

  return (
    <>
      <button
        type="button"
        className="ks-toggle"
        title="Expand knowledge panel"
        onClick={(e) => {
          const host = document.getElementById('knowledge-panel');
          const nowCollapsed = Boolean(host?.classList.toggle('ks-collapsed'));
          e.currentTarget.textContent = nowCollapsed ? '‹' : '›';
          setCollapsed(nowCollapsed);
        }}
      >
        {collapsed ? '‹' : '›'}
      </button>
      <div className="ks-inner">
        <div className="ks-panel-title">Related knowledge</div>
        <div className="ks-search-bar">
          <input
            className="ks-input"
            type="text"
            placeholder="Search knowledge…"
            autoComplete="off"
            value={query}
            onChange={(e) => onQueryChange(e.target.value)}
            onInput={(e) => onQueryChange((e.target as HTMLInputElement).value)}
          />
        </div>
        <div className="ks-results">
          {results.kind === 'hits'
            ? results.hits.map((hit, i) => {
                const title = hit.title || hit.path || '';
                const repo = (hit.repo || '').split('/').pop() || '';
                const url = hit.url || '#';
                return (
                  <a key={`${hit.repo || ''}-${hit.path || ''}-${i}`} className="ks-hit" href={url} target="_blank" rel="noopener noreferrer">
                    <div className="ks-hit-title">{title}</div>
                    <span className="ks-hit-repo">{repo}</span>
                    <div className="ks-hit-snippet" dangerouslySetInnerHTML={{ __html: getSnippet(hit) }} />
                  </a>
                );
              })
            : results.text ? <div className="ks-status-msg">{results.text}</div> : null}
        </div>
      </div>
    </>
  );
}

export function KnowledgeSearchHost({
  entry,
  creating,
}: {
  entry?: { title?: string; common_path?: string } | null;
  creating?: boolean;
}) {
  return (
    <div
      id="knowledge-panel"
      className={creating ? 'ks-collapsed viewer-chrome-persisted' : 'ks-collapsed viewer-chrome-persisted'}
      style={{ display: creating ? 'none' : undefined }}
    >
      {!creating ? <KnowledgeSearchPanel entry={entry} /> : null}
    </div>
  );
}

const panelRoots = new WeakMap<HTMLElement, Root>();

export function mountKnowledgeSearch(container: HTMLElement | null) {
  if (!container) return;
  let root = panelRoots.get(container);
  if (!root) {
    root = createRoot(container);
    panelRoots.set(container, root);
  }
  flushSync(() => {
    root!.render(<KnowledgeSearchPanel />);
  });
}

export function triggerKnowledgeSearch(entry: { title?: string; common_path?: string }) {
  const q = buildQuery(entry);
  const input = document.querySelector('.ks-input') as HTMLInputElement | null;
  if (input) input.value = q;
  if (liveTrigger) liveTrigger(q);
  else triggerStore.set({ query: q, token: triggerStore.getSnapshot().token + 1 });
}
