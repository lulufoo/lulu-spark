// @ts-nocheck — ported from JS; _state bookkeeping stays unused like the original.
import type { ReactNode } from 'react';
import { searchKnowledge, reindexKnowledge, getReindexStatus } from '../host/api.ts';
import { renderToHtml } from '../island.ts';

let _container: HTMLElement | null = null;
let _state = 'idle';
let _pollTimer: ReturnType<typeof setInterval> | null = null;
let _debounceTimer: ReturnType<typeof setTimeout> | null = null;

type KbHit = {
  title?: string;
  path?: string;
  repo?: string;
  url?: string;
  body?: string;
  _formatted?: { body?: string };
};

function KnowledgeSearchSkeleton() {
  return (
    <>
      <button type="button" className="ks-toggle" title="Expand knowledge panel">
        ‹
      </button>
      <div className="ks-inner">
        <div className="ks-panel-title">
          Related knowledge
          <button type="button" className="ks-refresh-btn" title="Sync knowledge">
            ↺
          </button>
        </div>
        <div className="ks-search-bar">
          <input className="ks-input" type="text" placeholder="Search knowledge…" autoComplete="off" />
        </div>
        <div className="ks-sync-bar" style={{ display: 'none' }}>
          ⟳ Syncing knowledge…
        </div>
        <div className="ks-results">
          <div className="ks-status-msg" />
        </div>
      </div>
    </>
  );
}

function KsHitCard({ hit }: { hit: KbHit }) {
  const title = hit.title || hit.path || '';
  const repo = (hit.repo || '').split('/').pop() || '';
  const url = hit.url || '#';
  return (
    <a className="ks-hit" href={url} target="_blank" rel="noopener noreferrer">
      <div className="ks-hit-title">{title}</div>
      <span className="ks-hit-repo">{repo}</span>
      <div className="ks-hit-snippet" dangerouslySetInnerHTML={{ __html: _getSnippet(hit) }} />
    </a>
  );
}

export function mountKnowledgeSearch(container: HTMLElement | null) {
  if (!container) return;
  _container = container;
  _container.innerHTML = renderToHtml(<KnowledgeSearchSkeleton />);
  const toggleBtn = _container.querySelector('.ks-toggle');
  if (toggleBtn) toggleBtn.textContent = _container.classList.contains('ks-collapsed') ? '‹' : '›';
  _bindEvents();
}

export function triggerKnowledgeSearch(entry: { title?: string; common_path?: string }) {
  if (!_container) return;
  const q = _buildQuery(entry);
  const input = _container.querySelector('.ks-input') as HTMLInputElement | null;
  if (input) input.value = q;
  void _search(q);
}

function _buildQuery(entry: { title?: string; common_path?: string }) {
  const parts: string[] = [];
  if (entry.title && entry.title.trim()) parts.push(entry.title.trim());
  const cp = entry.common_path || '';
  const filename = cp.split('/').pop() || '';
  const slug = filename.replace(/\.md$/, '').replace(/^\d{8,14}-/, '');
  if (slug && slug !== entry.title) parts.push(slug);
  return parts.join(' ');
}

function _bindEvents() {
  if (!_container) return;
  const input = _container.querySelector('.ks-input') as HTMLInputElement | null;
  if (input) {
    input.addEventListener('input', () => {
      if (_debounceTimer != null) clearTimeout(_debounceTimer);
      _debounceTimer = setTimeout(() => {
        void _search(input.value.trim());
      }, 300);
    });
  }

  const refreshBtn = _container.querySelector('.ks-refresh-btn');
  if (refreshBtn) {
    refreshBtn.addEventListener('click', () => {
      void _startSync();
    });
  }

  const toggleBtn = _container.querySelector('.ks-toggle');
  if (toggleBtn) {
    toggleBtn.addEventListener('click', _togglePanel);
  }
}

function _togglePanel() {
  if (!_container) return;
  const btn = _container.querySelector('.ks-toggle');
  const collapsed = _container.classList.toggle('ks-collapsed');
  if (btn) btn.textContent = collapsed ? '‹' : '›';
}

async function _search(q: string) {
  if (!q) {
    _renderStatusMsg('Enter keywords to search');
    return;
  }

  _setState('loading');
  _renderStatusMsg('Searching…');

  try {
    const data = (await searchKnowledge(q, 10)) as { error?: string; hits?: KbHit[] };

    if (data.error === 'unavailable') {
      _setState('unavailable');
      _container?.classList.add('ks-unavailable');
      return;
    }

    _container?.classList.remove('ks-unavailable');

    if (data.error === 'not_indexed') {
      _renderStatusMsg(
        <>
          Knowledge index not built
          <br />
          <span style={{ fontSize: 10, color: '#aaa' }}>Click ↺ to sync knowledge</span>
        </>,
      );
      return;
    }

    const hits = data.hits || [];
    if (hits.length === 0) {
      _setState('empty');
      _renderStatusMsg(
        <>
          No related knowledge
          <br />
          <span style={{ fontSize: 10, color: '#aaa' }}>Try another keyword?</span>
        </>,
      );
    } else {
      _setState('showing');
      _renderHits(hits);
    }
  } catch {
    _renderStatusMsg('Search error');
  }
}

async function _startSync() {
  if (!_container) return;
  const syncBar = _container.querySelector('.ks-sync-bar') as HTMLElement | null;
  const refreshBtn = _container.querySelector('.ks-refresh-btn') as HTMLButtonElement | null;
  if (refreshBtn) refreshBtn.disabled = true;
  if (syncBar) {
    syncBar.className = 'ks-sync-bar';
    syncBar.textContent = '⟳ Syncing knowledge…';
    syncBar.style.display = 'block';
  }

  try {
    const res = (await reindexKnowledge()) as { error?: string };
    if (res.error) {
      _stopSync(true, res.error);
      return;
    }
  } catch {
    _stopSync(true, 'Request failed');
    return;
  }

  _setState('syncing');
  _pollTimer = setInterval(() => {
    void _pollSync();
  }, 2000);
}

async function _pollSync() {
  try {
    const res = (await getReindexStatus()) as { status?: string; log?: string };
    if (res.status === 'done') {
      _stopSync(false);
      const input = _container?.querySelector('.ks-input') as HTMLInputElement | null;
      if (input && input.value.trim()) void _search(input.value.trim());
    } else if (res.status === 'error') {
      _stopSync(true, res.log || 'Sync failed');
    }
  } catch {
    // keep polling
  }
}

function _stopSync(isError: boolean, msg?: string) {
  if (_pollTimer != null) clearInterval(_pollTimer);
  _pollTimer = null;

  const refreshBtn = _container?.querySelector('.ks-refresh-btn') as HTMLButtonElement | null;
  if (refreshBtn) refreshBtn.disabled = false;

  const syncBar = _container?.querySelector('.ks-sync-bar') as HTMLElement | null;
  if (isError) {
    if (syncBar) {
      syncBar.className = 'ks-sync-bar ks-error';
      syncBar.textContent = `⚠ Sync failed: ${msg}`;
    }
  } else if (syncBar) {
    syncBar.style.display = 'none';
  }
}

function _renderHits(hits: KbHit[]) {
  const resultsEl = _container?.querySelector('.ks-results');
  if (!resultsEl) return;
  resultsEl.innerHTML = renderToHtml(
    <>
      {hits.map((hit, i) => (
        <KsHitCard key={`${hit.repo || ''}-${hit.path || ''}-${i}`} hit={hit} />
      ))}
    </>,
  );
}

function _getSnippet(hit: KbHit) {
  const formatted = hit._formatted || {};
  const body = formatted.body || hit.body || '';
  const plain = body.replace(/^#{1,6}\s+/gm, '').replace(/[*_`]/g, '');
  const snippet = plain.slice(0, 160).trim();
  return _escKeepEm(snippet);
}

function _setState(s: string) {
  _state = s;
}

function _renderStatusMsg(node: ReactNode) {
  const resultsEl = _container?.querySelector('.ks-results');
  if (resultsEl) {
    resultsEl.innerHTML = renderToHtml(<div className="ks-status-msg">{node}</div>);
  }
}

function _escKeepEm(str: string) {
  return str
    .replace(/&/g, '&amp;')
    .replace(/<em>/g, '\x00EM\x00')
    .replace(/<\/em>/g, '\x00_EM\x00')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/\x00EM\x00/g, '<em>')
    .replace(/\x00_EM\x00/g, '</em>');
}
