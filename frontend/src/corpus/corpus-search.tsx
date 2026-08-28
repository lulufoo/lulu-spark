import type { ReactNode } from 'react';
import { searchKnowledge, reindexKnowledge, getReindexStatus } from '../host/api.ts';
import { renderToHtml } from '../island.ts';

let _initialized = false;
let _debounceTimer: ReturnType<typeof setTimeout> | null = null;
let _pollTimer: ReturnType<typeof setInterval> | null = null;
let _inputFocused = false;

const _HIST_KEY = 'gs-history-kb';
const _HIST_MAX = 10;

type KbHit = {
  title?: string;
  path?: string;
  repo?: string;
  url?: string;
  body?: string;
  _formatted?: { body?: string };
};

export function initCorpusSearch() {
  if (_initialized) return;
  _initialized = true;

  const input = document.getElementById('gs-kb-input') as HTMLInputElement | null;
  const rebuildBtn = document.getElementById('gs-kb-rebuild-btn');
  if (!input) return;

  input.addEventListener('input', () => {
    if (_debounceTimer) clearTimeout(_debounceTimer);
    const q = _normalizeQuery(input.value);
    _updateRebuildUI();
    if (!q) {
      _showHistory();
      return;
    }
    _debounceTimer = setTimeout(() => {
      void _search(q);
    }, 300);
  });

  input.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      _close();
      input.blur();
    }
  });

  input.addEventListener('focus', () => {
    _inputFocused = true;
    _updateRebuildUI();
    if (!input.value) _showHistory();
  });

  input.addEventListener('blur', () => {
    _inputFocused = false;
    setTimeout(() => _updateRebuildUI(), 200);
  });

  if (rebuildBtn) {
    rebuildBtn.addEventListener('click', () => {
      void _startKbRebuild();
    });
  }

  document.addEventListener('click', (e) => {
    const wrap = document.getElementById('gs-kb-wrap');
    if (wrap && !wrap.contains(e.target as Node)) _close();
  });
}

export function closeCorpusSearch() {
  _close();
}

function _normalizeQuery(raw: string) {
  return String(raw || '').trim();
}

function _updateRebuildUI() {
  const rebuildBtn = document.getElementById('gs-kb-rebuild-btn');
  if (rebuildBtn && _pollTimer === null) {
    rebuildBtn.style.display = _inputFocused ? 'inline-flex' : 'none';
  }
}

function GsStatus({ children, color }: { children: ReactNode; color?: string }) {
  return (
    <div className="gs-status" style={color ? { color } : undefined}>
      {children}
    </div>
  );
}

function KbHits({ hits }: { hits: KbHit[] }) {
  return (
    <>
      {hits.map((hit, i) => (
        <div
          key={`${hit.repo || ''}-${hit.path || ''}-${i}`}
          className="gs-hit gs-hit-kb"
          data-repo={hit.repo || ''}
          data-path={hit.path || ''}
          data-url={hit.url || ''}
          data-title={hit.title || hit.path || ''}
        >
          <div className="gs-hit-title">{hit.title || hit.path || ''}</div>
          <span className="gs-hit-repo">{(hit.repo || '').split('/').pop()}</span>
          <div className="gs-hit-snippet" dangerouslySetInnerHTML={{ __html: _getSnippet(hit) }} />
        </div>
      ))}
    </>
  );
}

function HistoryList({ items }: { items: string[] }) {
  return (
    <div className="gs-hist-list">
      {items.map((q) => (
        <div key={q} className="gs-hist-item" data-q={q}>
          <span className="gs-hist-label">{q}</span>
          <button type="button" className="gs-hist-remove" data-q={q} title="Delete">
            ×
          </button>
        </div>
      ))}
    </div>
  );
}

async function _search(q: string) {
  const dropdown = document.getElementById('gs-kb-dropdown');
  if (!dropdown) return;

  _show(dropdown, renderToHtml(<GsStatus>Searching…</GsStatus>));

  try {
    const data = (await searchKnowledge(q, 8)) as { error?: string; hits?: KbHit[] };

    if (data.error === 'unavailable') {
      _show(
        dropdown,
        renderToHtml(
          <GsStatus>
            Meilisearch is not running; search unavailable
            <br />
            <span style={{ fontSize: 10, opacity: 0.7 }}>
              Start external Meilisearch first (default localhost:7700)
            </span>
          </GsStatus>,
        ),
      );
      return;
    }
    if (data.error === 'not_indexed') {
      _show(dropdown, renderToHtml(<GsStatus>Index not built yet. Click ↺ to rebuild.</GsStatus>));
      return;
    }
    const hits = data.hits || [];
    if (hits.length === 0) {
      _show(dropdown, renderToHtml(<GsStatus>No related results</GsStatus>));
    } else {
      _show(dropdown, renderToHtml(<KbHits hits={hits} />));
    }
  } catch {
    _show(dropdown, renderToHtml(<GsStatus>Search error</GsStatus>));
  }
}

async function _startKbRebuild() {
  const btn = document.getElementById('gs-kb-rebuild-btn') as HTMLButtonElement | null;
  if (btn) {
    btn.disabled = true;
    btn.classList.add('syncing');
  }

  try {
    const res = (await reindexKnowledge()) as { error?: string };
    if (res.error) {
      _stopKbRebuild(true, res.error);
      return;
    }
  } catch {
    _stopKbRebuild(true, 'Request failed');
    return;
  }

  _pollTimer = setInterval(() => {
    void _pollKbRebuild();
  }, 2000);
}

async function _pollKbRebuild() {
  try {
    const res = (await getReindexStatus()) as { status?: string; log?: string };
    if (res.status === 'done') {
      _stopKbRebuild(false, res.log);
    } else if (res.status === 'error') {
      _stopKbRebuild(true, res.log || 'Rebuild failed');
    }
  } catch {
    // keep polling
  }
}

function _stopKbRebuild(isError: boolean, msg?: string) {
  if (_pollTimer) clearInterval(_pollTimer);
  _pollTimer = null;
  const btn = document.getElementById('gs-kb-rebuild-btn') as HTMLButtonElement | null;
  if (btn) {
    btn.disabled = false;
    btn.classList.remove('syncing');
    btn.title = isError ? `Rebuild failed: ${msg}` : 'Rebuild knowledge index';
  }
  _updateRebuildUI();
  const dropdown = document.getElementById('gs-kb-dropdown');
  if (dropdown) {
    const color = isError ? '#cf222e' : '#1a7f37';
    _show(
      dropdown,
      renderToHtml(
        <GsStatus color={color}>
          {isError ? '❌ Rebuild failed: ' : '✅ Rebuild finished: '}
          {msg || (isError ? 'Unknown error' : '')}
        </GsStatus>,
      ),
    );
  }
}

function _getHistory(): string[] {
  try {
    return JSON.parse(localStorage.getItem(_HIST_KEY) || '') || [];
  } catch {
    return [];
  }
}

function _addHistory(q: string) {
  const normalized = _normalizeQuery(q);
  if (!normalized) return;
  const list = _getHistory().filter((x) => x !== normalized);
  list.unshift(normalized);
  localStorage.setItem(_HIST_KEY, JSON.stringify(list.slice(0, _HIST_MAX)));
}

function _removeHistory(q: string) {
  const list = _getHistory().filter((x) => x !== q);
  localStorage.setItem(_HIST_KEY, JSON.stringify(list));
}

function _showHistory() {
  const dropdown = document.getElementById('gs-kb-dropdown');
  if (!dropdown) return;
  const list = _getHistory();
  if (!list.length) return;
  _show(dropdown, renderToHtml(<HistoryList items={list} />));
  dropdown.querySelectorAll('.gs-hist-item').forEach((el) => {
    el.addEventListener('click', (e) => {
      if ((e.target as Element).classList.contains('gs-hist-remove')) return;
      const input = document.getElementById('gs-kb-input') as HTMLInputElement | null;
      if (!input) return;
      input.value = (el as HTMLElement).dataset.q || '';
      input.dispatchEvent(new Event('input', { bubbles: true }));
    });
  });
  dropdown.querySelectorAll('.gs-hist-remove').forEach((btn) => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      _removeHistory((btn as HTMLElement).dataset.q || '');
      _showHistory();
    });
  });
}

function _getSnippet(hit: KbHit) {
  const formatted = hit._formatted || {};
  const body = formatted.body || hit.body || '';
  const plain = body.replace(/^#{1,6}\s+/gm, '').replace(/[*_`]/g, '');
  return _escKeepEm(plain.slice(0, 120).trim());
}

function _show(dropdown: HTMLElement, html: string) {
  dropdown.innerHTML = html;
  dropdown.style.display = 'block';

  dropdown.querySelectorAll('.gs-hit-kb').forEach((el) => {
    el.addEventListener('click', () => {
      const node = el as HTMLElement;
      const repo = node.dataset.repo;
      const path = node.dataset.path;
      const url = node.dataset.url;
      const title = node.dataset.title;
      if (repo && path) {
        document.dispatchEvent(
          new CustomEvent('cta:open-kb-doc', {
            detail: { repo, path, url, title },
          }),
        );
      }
      const input = document.getElementById('gs-kb-input') as HTMLInputElement | null;
      _addHistory(_normalizeQuery(input?.value || ''));
      _close();
    });
  });
}

function _close() {
  const dropdown = document.getElementById('gs-kb-dropdown');
  if (dropdown) dropdown.style.display = 'none';
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
