import type { ReactNode } from 'react';
import { searchWorkbench, reindexWorkbench, getReindexWorkbenchStatus } from '../host/api.ts';
import { renderToHtml } from '../island.ts';

let _initialized = false;
let _debounceTimer: ReturnType<typeof setTimeout> | null = null;
let _pollTimer: ReturnType<typeof setInterval> | null = null;
let _inputFocused = false;

const _HIST_KEY = 'gs-history-wb';
const _HIST_MAX = 10;

const _LAYER_LABEL: Record<string, string> = {
  raw: 'Original',
  distilled: 'Distilled',
  digest: 'Summary',
  diagnose: 'Diagnose',
};

type WbHit = {
  title?: string;
  common_path?: string;
  layer?: string;
  topic?: string;
  body?: string;
  _formatted?: { body?: string };
};

export function initWorkbenchSearch() {
  if (_initialized) return;
  _initialized = true;

  const input = document.getElementById('gs-wb-input') as HTMLInputElement | null;
  const rebuildBtn = document.getElementById('gs-wb-rebuild-btn');
  if (!input) return;

  input.addEventListener('input', () => {
    if (_debounceTimer) clearTimeout(_debounceTimer);
    const q = _normalizeQuery(input.value);
    _updateRebuildUI();
    if (!q) {
      _showHistory();
      return;
    }
    _debounceTimer = setTimeout(() => _search(q), 300);
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
    rebuildBtn.addEventListener('click', _startWbRebuild);
  }

  document.addEventListener('click', (e) => {
    const wrap = document.getElementById('gs-wb-wrap');
    if (wrap && !wrap.contains(e.target as Node)) _close();
  });
}

export function closeWorkbenchSearch() {
  _close();
}

function _normalizeQuery(raw: unknown) {
  const trimmed = String(raw || '').trim();
  return trimmed.startsWith('#') ? trimmed.slice(1).trim() : trimmed;
}

function _updateRebuildUI() {
  const rebuildBtn = document.getElementById('gs-wb-rebuild-btn');
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

async function _search(q: string) {
  const dropdown = document.getElementById('gs-wb-dropdown');
  if (!dropdown) return;

  _show(dropdown, renderToHtml(<GsStatus>Searching…</GsStatus>));

  try {
    const data = await searchWorkbench(q, 8);

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
    const hits = (data.hits || []) as WbHit[];
    if (hits.length === 0) {
      _show(dropdown, renderToHtml(<GsStatus>No related results</GsStatus>));
    } else {
      _show(dropdown, renderToHtml(<WbHits hits={hits} />));
    }
  } catch {
    _show(dropdown, renderToHtml(<GsStatus>Search error</GsStatus>));
  }
}

function WbHits({ hits }: { hits: WbHit[] }) {
  return (
    <>
      {hits.map((hit, i) => {
        const title = hit.title || hit.common_path || '';
        const layer = hit.layer || '';
        const topic = hit.topic || '';
        const cp = hit.common_path || '';
        const snippet = _getSnippet(hit);
        return (
          <div key={`${cp}-${layer}-${i}`} className="gs-hit gs-hit-wb" data-common-path={cp} data-layer={layer}>
            <div className="gs-hit-title">{title}</div>
            <span className={`gs-hit-layer gs-layer-${layer}`}>{_LAYER_LABEL[layer] || layer}</span>
            <span className="gs-hit-repo">{topic}</span>
            <div className="gs-hit-snippet" dangerouslySetInnerHTML={{ __html: snippet }} />
          </div>
        );
      })}
    </>
  );
}

async function _startWbRebuild() {
  const btn = document.getElementById('gs-wb-rebuild-btn') as HTMLButtonElement | null;
  if (btn) {
    btn.disabled = true;
    btn.classList.add('syncing');
  }

  try {
    const res = await reindexWorkbench();
    if (res.error) {
      _stopWbRebuild(true, res.error);
      return;
    }
  } catch {
    _stopWbRebuild(true, 'Request failed');
    return;
  }

  _pollTimer = setInterval(_pollWbRebuild, 2000);
}

async function _pollWbRebuild() {
  try {
    const res = await getReindexWorkbenchStatus();
    if (res.status === 'done') {
      _stopWbRebuild(false, res.log);
    } else if (res.status === 'error') {
      _stopWbRebuild(true, res.log || 'Rebuild failed');
    }
  } catch {
    /* keep polling */
  }
}

function _stopWbRebuild(isError: boolean, msg: unknown) {
  if (_pollTimer) clearInterval(_pollTimer);
  _pollTimer = null;
  const btn = document.getElementById('gs-wb-rebuild-btn') as HTMLButtonElement | null;
  if (btn) {
    btn.disabled = false;
    btn.classList.remove('syncing');
    btn.title = isError ? `Rebuild failed: ${msg}` : 'Rebuild Workbench index';
  }
  _updateRebuildUI();
  const dropdown = document.getElementById('gs-wb-dropdown');
  if (dropdown) {
    const color = isError ? '#cf222e' : '#1a7f37';
    _show(
      dropdown,
      renderToHtml(
        <GsStatus color={color}>
          {isError ? '❌ Rebuild failed: ' : '✅ Rebuild finished: '}
          {String(msg || (isError ? 'Unknown error' : ''))}
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

function HistoryList({ queries }: { queries: string[] }) {
  return (
    <div className="gs-hist-list">
      {queries.map((q) => (
        <div key={q} className="gs-hist-item" data-q={q}>
          <span className="gs-hist-label">{q}</span>
          <button className="gs-hist-remove" data-q={q} title="Delete">
            ×
          </button>
        </div>
      ))}
    </div>
  );
}

function _renderHistory() {
  const list = _getHistory();
  if (!list.length) return '';
  return renderToHtml(<HistoryList queries={list} />);
}

function _showHistory() {
  const dropdown = document.getElementById('gs-wb-dropdown');
  if (!dropdown) return;
  const html = _renderHistory();
  if (!html) return;
  _show(dropdown, html);
  dropdown.querySelectorAll('.gs-hist-item').forEach((el) => {
    el.addEventListener('click', (e) => {
      if ((e.target as HTMLElement).classList.contains('gs-hist-remove')) return;
      const input = document.getElementById('gs-wb-input') as HTMLInputElement | null;
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

function _getSnippet(hit: WbHit) {
  const formatted = hit._formatted || {};
  const body = formatted.body || hit.body || '';
  const plain = body.replace(/^#{1,6}\s+/gm, '').replace(/[*_`]/g, '');
  return _escKeepEm(plain.slice(0, 120).trim());
}

function _show(dropdown: HTMLElement, html: string) {
  dropdown.innerHTML = html;
  dropdown.style.display = 'block';

  dropdown.querySelectorAll('.gs-hit-wb').forEach((el) => {
    el.addEventListener('click', () => {
      const cp = (el as HTMLElement).dataset.commonPath;
      if (cp) {
        const detail: { common_path: string; layer?: string } = { common_path: cp };
        const layer = (el as HTMLElement).dataset.layer;
        if (layer) detail.layer = layer;
        document.dispatchEvent(new CustomEvent('cta:open-entry', { detail }));
      }
      const input = document.getElementById('gs-wb-input') as HTMLInputElement | null;
      _addHistory(_normalizeQuery(input?.value || ''));
      _close();
    });
  });
}

function _close() {
  const dropdown = document.getElementById('gs-wb-dropdown');
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
