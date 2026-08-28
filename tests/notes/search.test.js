// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const apiMocks = vi.hoisted(() => ({
  searchWorkbench: vi.fn(),
  searchKnowledge: vi.fn(),
  reindexWorkbench: vi.fn(),
  getReindexWorkbenchStatus: vi.fn(),
}));

vi.mock('../../frontend/src/host/api.ts', () => ({
  searchWorkbench: (...args) => apiMocks.searchWorkbench(...args),
  searchKnowledge: (...args) => apiMocks.searchKnowledge(...args),
  reindexWorkbench: (...args) => apiMocks.reindexWorkbench(...args),
  getReindexWorkbenchStatus: (...args) => apiMocks.getReindexWorkbenchStatus(...args),
}));

function seedWorkbenchSearchDom() {
  document.body.innerHTML = `
    <div id="gs-wb-wrap" class="gs-search-wrap">
      <input id="gs-wb-input" class="gs-search-input" type="text" autocomplete="off" />
      <button id="gs-wb-rebuild-btn" class="gs-rebuild-btn" style="display:none" title="Rebuild Workbench index">↺</button>
      <div id="gs-wb-dropdown" class="gs-search-dropdown" style="display:none"></div>
    </div>
    <button id="outside-click-target">outside</button>
  `;
}

function installLocalStorageMock() {
  const store = {};
  globalThis.localStorage = {
    getItem(key) {
      return Object.prototype.hasOwnProperty.call(store, key) ? store[key] : null;
    },
    setItem(key, value) {
      store[key] = String(value);
    },
    removeItem(key) {
      delete store[key];
    },
  };
}

async function loadModule() {
  vi.resetModules();
  return import('../../frontend/src/notes/search.tsx');
}

describe('notes search module', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    installLocalStorageMock();
    seedWorkbenchSearchDom();
    apiMocks.searchWorkbench.mockResolvedValue({ hits: [] });
    apiMocks.reindexWorkbench.mockResolvedValue({});
    apiMocks.getReindexWorkbenchStatus.mockResolvedValue({ status: 'running' });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('exports initWorkbenchSearch and closeWorkbenchSearch', async () => {
    const mod = await loadModule();
    expect(typeof mod.initWorkbenchSearch).toBe('function');
    expect(typeof mod.closeWorkbenchSearch).toBe('function');
  });

  it('debounced input calls searchWorkbench(q, 8) only', async () => {
    const { initWorkbenchSearch } = await loadModule();
    initWorkbenchSearch();

    const input = document.getElementById('gs-wb-input');
    input.value = 'alpha';
    input.dispatchEvent(new Event('input', { bubbles: true }));

    await vi.advanceTimersByTimeAsync(300);

    expect(apiMocks.searchWorkbench).toHaveBeenCalledWith('alpha', 8);
    expect(apiMocks.searchKnowledge).not.toHaveBeenCalled();
  });

  it('strips leading # before calling searchWorkbench', async () => {
    const { initWorkbenchSearch } = await loadModule();
    initWorkbenchSearch();

    const input = document.getElementById('gs-wb-input');
    input.value = '#topic-name';
    input.dispatchEvent(new Event('input', { bubbles: true }));

    await vi.advanceTimersByTimeAsync(300);

    expect(apiMocks.searchWorkbench).toHaveBeenCalledWith('topic-name', 8);
  });

  it('hit click dispatches cta:open-entry with common_path and data-layer', async () => {
    apiMocks.searchWorkbench.mockResolvedValue({
      hits: [{
        title: 'Entry',
        common_path: '2024/01/entry.md',
        layer: 'raw',
        topic: 'demo',
        body: 'snippet body',
      }],
    });

    const { initWorkbenchSearch } = await loadModule();
    initWorkbenchSearch();

    const input = document.getElementById('gs-wb-input');
    input.value = 'entry';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    const handler = vi.fn();
    document.addEventListener('cta:open-entry', handler);

    const hit = document.querySelector('.gs-hit-wb');
    expect(hit).toBeTruthy();
    hit.click();

    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler.mock.calls[0][0].detail).toEqual({
      common_path: '2024/01/entry.md',
      layer: 'raw',
    });
  });

  it('rebuild button calls reindexWorkbench and polls status', async () => {
    apiMocks.getReindexWorkbenchStatus
      .mockResolvedValueOnce({ status: 'running' })
      .mockResolvedValueOnce({ status: 'done', log: 'ok' });

    const { initWorkbenchSearch } = await loadModule();
    initWorkbenchSearch();

    document.getElementById('gs-wb-rebuild-btn').click();
    await Promise.resolve();

    expect(apiMocks.reindexWorkbench).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(2000);
    expect(apiMocks.getReindexWorkbenchStatus).toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(2000);
    expect(apiMocks.getReindexWorkbenchStatus.mock.calls.length).toBeGreaterThanOrEqual(2);
  });

  it('stores history under gs-history-wb without # prefix', async () => {
    apiMocks.searchWorkbench.mockResolvedValue({
      hits: [{
        title: 'Hit',
        common_path: 'path/a.md',
        layer: 'digest',
        topic: 't',
        body: 'body',
      }],
    });

    const { initWorkbenchSearch } = await loadModule();
    initWorkbenchSearch();

    const input = document.getElementById('gs-wb-input');
    input.value = '#stored-query';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    document.querySelector('.gs-hit-wb').click();

    const stored = JSON.parse(localStorage.getItem('gs-history-wb') || '[]');
    expect(stored).toContain('stored-query');
    expect(stored.some((q) => q.startsWith('#'))).toBe(false);
  });

  it('shows global-search-equivalent status for unavailable', async () => {
    apiMocks.searchWorkbench.mockResolvedValue({ error: 'unavailable' });

    const { initWorkbenchSearch } = await loadModule();
    initWorkbenchSearch();

    document.getElementById('gs-wb-input').value = 'q';
    document.getElementById('gs-wb-input').dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    const dropdown = document.getElementById('gs-wb-dropdown');
    expect(dropdown.style.display).toBe('block');
    expect(dropdown.textContent).toContain('Meilisearch is not running');
  });

  it('shows global-search-equivalent status for not_indexed', async () => {
    apiMocks.searchWorkbench.mockResolvedValue({ error: 'not_indexed' });

    const { initWorkbenchSearch } = await loadModule();
    initWorkbenchSearch();

    document.getElementById('gs-wb-input').value = 'q';
    document.getElementById('gs-wb-input').dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    const dropdown = document.getElementById('gs-wb-dropdown');
    expect(dropdown.textContent).toContain('Index not built yet');
  });

  it('shows no-results and search-error statuses', async () => {
    const { initWorkbenchSearch } = await loadModule();
    initWorkbenchSearch();

    document.getElementById('gs-wb-input').value = 'empty';
    document.getElementById('gs-wb-input').dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);
    expect(document.getElementById('gs-wb-dropdown').textContent).toContain('No related results');

    apiMocks.searchWorkbench.mockRejectedValue(new Error('network'));
    document.getElementById('gs-wb-input').value = 'err';
    document.getElementById('gs-wb-input').dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);
    expect(document.getElementById('gs-wb-dropdown').textContent).toContain('Search error');
  });

  it('Escape closes dropdown and blurs input', async () => {
    const { initWorkbenchSearch } = await loadModule();
    initWorkbenchSearch();

    const input = document.getElementById('gs-wb-input');
    const dropdown = document.getElementById('gs-wb-dropdown');
    dropdown.style.display = 'block';

    input.focus();
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));

    expect(dropdown.style.display).toBe('none');
    expect(document.activeElement).not.toBe(input);
  });

  it('document click outside #gs-wb-wrap closes dropdown', async () => {
    const { initWorkbenchSearch } = await loadModule();
    initWorkbenchSearch();

    const dropdown = document.getElementById('gs-wb-dropdown');
    dropdown.style.display = 'block';

    document.getElementById('outside-click-target').click();

    expect(dropdown.style.display).toBe('none');
  });

  it('closeWorkbenchSearch hides #gs-wb-dropdown', async () => {
    const { closeWorkbenchSearch } = await loadModule();
    const dropdown = document.getElementById('gs-wb-dropdown');
    dropdown.style.display = 'block';

    closeWorkbenchSearch();

    expect(dropdown.style.display).toBe('none');
  });

  it('initWorkbenchSearch is idempotent (no duplicate search on one input)', async () => {
    const { initWorkbenchSearch } = await loadModule();
    initWorkbenchSearch();
    initWorkbenchSearch();

    const input = document.getElementById('gs-wb-input');
    input.value = 'once';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    expect(apiMocks.searchWorkbench).toHaveBeenCalledTimes(1);
  });
});
