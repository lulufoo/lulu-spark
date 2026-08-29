// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const apiMocks = vi.hoisted(() => ({
  searchKnowledge: vi.fn(),
  searchWorkbench: vi.fn(),
  reindexKnowledge: vi.fn(),
  getReindexStatus: vi.fn(),
}));

vi.mock('../../frontend/src/host/api.ts', () => ({
  searchKnowledge: (...args) => apiMocks.searchKnowledge(...args),
  searchWorkbench: (...args) => apiMocks.searchWorkbench(...args),
  reindexKnowledge: (...args) => apiMocks.reindexKnowledge(...args),
  getReindexStatus: (...args) => apiMocks.getReindexStatus(...args),
}));

function seedKnowledgeSearchDom() {
  document.body.innerHTML = `
    <div id="gs-kb-wrap" class="gs-search-wrap">
      <input id="gs-kb-input" class="gs-search-input" type="text" autocomplete="off" />
      <button id="gs-kb-rebuild-btn" class="gs-rebuild-btn" style="display:none" title="重建知识库索引">↺</button>
      <div id="gs-kb-dropdown" class="gs-search-dropdown" style="display:none"></div>
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
  return import('../../frontend/src/knowledge/ui/search.tsx');
}

describe('knowledge-search module', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    installLocalStorageMock();
    seedKnowledgeSearchDom();
    apiMocks.searchKnowledge.mockResolvedValue({ hits: [] });
    apiMocks.reindexKnowledge.mockResolvedValue({});
    apiMocks.getReindexStatus.mockResolvedValue({ status: 'running' });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('exports initKnowledgeSearch and closeKnowledgeSearch', async () => {
    const mod = await loadModule();
    expect(typeof mod.initKnowledgeSearch).toBe('function');
    expect(typeof mod.closeKnowledgeSearch).toBe('function');
  });

  it('debounced input calls searchKnowledge(q, 8) only', async () => {
    const { initKnowledgeSearch } = await loadModule();
    initKnowledgeSearch();

    const input = document.getElementById('gs-kb-input');
    input.value = 'alpha';
    input.dispatchEvent(new Event('input', { bubbles: true }));

    await vi.advanceTimersByTimeAsync(300);

    expect(apiMocks.searchKnowledge).toHaveBeenCalledWith('alpha', 8);
    expect(apiMocks.searchWorkbench).not.toHaveBeenCalled();
  });

  it('hit click dispatches cta:open-kb-doc with repo, path, url, title', async () => {
    apiMocks.searchKnowledge.mockResolvedValue({
      hits: [{
        title: 'Doc Title',
        repo: 'org/repo',
        path: 'docs/readme.md',
        url: 'https://example.com/doc',
        body: 'snippet body',
      }],
    });

    const { initKnowledgeSearch } = await loadModule();
    initKnowledgeSearch();

    const input = document.getElementById('gs-kb-input');
    input.value = 'doc';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    const handler = vi.fn();
    document.addEventListener('cta:open-kb-doc', handler);

    const hit = document.querySelector('.gs-hit-kb');
    expect(hit).toBeTruthy();
    hit.click();

    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler.mock.calls[0][0].detail).toEqual({
      repo: 'org/repo',
      path: 'docs/readme.md',
      url: 'https://example.com/doc',
      title: 'Doc Title',
    });
  });

  it('rebuild button calls reindexKnowledge and polls getReindexStatus', async () => {
    apiMocks.getReindexStatus
      .mockResolvedValueOnce({ status: 'running' })
      .mockResolvedValueOnce({ status: 'done', log: 'ok' });

    const { initKnowledgeSearch } = await loadModule();
    initKnowledgeSearch();

    document.getElementById('gs-kb-rebuild-btn').click();
    await Promise.resolve();

    expect(apiMocks.reindexKnowledge).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(2000);
    expect(apiMocks.getReindexStatus).toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(2000);
    expect(apiMocks.getReindexStatus.mock.calls.length).toBeGreaterThanOrEqual(2);
  });

  it('stores history under gs-history-kb', async () => {
    apiMocks.searchKnowledge.mockResolvedValue({
      hits: [{
        title: 'Hit',
        repo: 'r',
        path: 'p.md',
        url: '',
        body: 'body',
      }],
    });

    const { initKnowledgeSearch } = await loadModule();
    initKnowledgeSearch();

    const input = document.getElementById('gs-kb-input');
    input.value = 'stored-query';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    document.querySelector('.gs-hit-kb').click();

    const stored = JSON.parse(localStorage.getItem('gs-history-kb') || '[]');
    expect(stored).toContain('stored-query');
  });

  it('shows rebuild button only when input is focused (focus-gated)', async () => {
    const { initKnowledgeSearch } = await loadModule();
    initKnowledgeSearch();

    const input = document.getElementById('gs-kb-input');
    const rebuildBtn = document.getElementById('gs-kb-rebuild-btn');

    expect(rebuildBtn.style.display).toBe('none');

    input.dispatchEvent(new Event('focus', { bubbles: true }));
    expect(rebuildBtn.style.display).toBe('inline-flex');

    input.dispatchEvent(new Event('blur', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(200);
    expect(rebuildBtn.style.display).toBe('none');
  });

  it('keeps rebuild button visible during rebuild polling', async () => {
    apiMocks.getReindexStatus.mockResolvedValue({ status: 'running' });

    const { initKnowledgeSearch } = await loadModule();
    initKnowledgeSearch();

    const input = document.getElementById('gs-kb-input');
    const rebuildBtn = document.getElementById('gs-kb-rebuild-btn');

    input.dispatchEvent(new Event('focus', { bubbles: true }));
    rebuildBtn.click();
    await Promise.resolve();

    input.dispatchEvent(new Event('blur', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(200);

    expect(rebuildBtn.style.display).not.toBe('none');
  });

  it('shows global-search-equivalent status for unavailable', async () => {
    apiMocks.searchKnowledge.mockResolvedValue({ error: 'unavailable' });

    const { initKnowledgeSearch } = await loadModule();
    initKnowledgeSearch();

    document.getElementById('gs-kb-input').value = 'q';
    document.getElementById('gs-kb-input').dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    const dropdown = document.getElementById('gs-kb-dropdown');
    expect(dropdown.style.display).toBe('block');
    expect(dropdown.textContent).toContain('Meilisearch is not running');
  });

  it('shows global-search-equivalent status for not_indexed', async () => {
    apiMocks.searchKnowledge.mockResolvedValue({ error: 'not_indexed' });

    const { initKnowledgeSearch } = await loadModule();
    initKnowledgeSearch();

    document.getElementById('gs-kb-input').value = 'q';
    document.getElementById('gs-kb-input').dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    const dropdown = document.getElementById('gs-kb-dropdown');
    expect(dropdown.textContent).toContain('Index not built yet');
  });

  it('shows no-results and search-error statuses', async () => {
    const { initKnowledgeSearch } = await loadModule();
    initKnowledgeSearch();

    document.getElementById('gs-kb-input').value = 'empty';
    document.getElementById('gs-kb-input').dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);
    expect(document.getElementById('gs-kb-dropdown').textContent).toContain('No related results');

    apiMocks.searchKnowledge.mockRejectedValue(new Error('network'));
    document.getElementById('gs-kb-input').value = 'err';
    document.getElementById('gs-kb-input').dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);
    expect(document.getElementById('gs-kb-dropdown').textContent).toContain('Search error');
  });

  it('Escape closes dropdown and blurs input', async () => {
    const { initKnowledgeSearch } = await loadModule();
    initKnowledgeSearch();

    const input = document.getElementById('gs-kb-input');
    const dropdown = document.getElementById('gs-kb-dropdown');
    dropdown.style.display = 'block';

    input.focus();
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));

    expect(dropdown.style.display).toBe('none');
    expect(document.activeElement).not.toBe(input);
  });

  it('document click outside #gs-kb-wrap closes dropdown', async () => {
    const { initKnowledgeSearch } = await loadModule();
    initKnowledgeSearch();

    const dropdown = document.getElementById('gs-kb-dropdown');
    dropdown.style.display = 'block';

    document.getElementById('outside-click-target').click();

    expect(dropdown.style.display).toBe('none');
  });

  it('closeKnowledgeSearch hides #gs-kb-dropdown', async () => {
    const { closeKnowledgeSearch } = await loadModule();
    const dropdown = document.getElementById('gs-kb-dropdown');
    dropdown.style.display = 'block';

    closeKnowledgeSearch();

    expect(dropdown.style.display).toBe('none');
  });

  it('initKnowledgeSearch is idempotent (no duplicate search on one input)', async () => {
    const { initKnowledgeSearch } = await loadModule();
    initKnowledgeSearch();
    initKnowledgeSearch();

    const input = document.getElementById('gs-kb-input');
    input.value = 'once';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    expect(apiMocks.searchKnowledge).toHaveBeenCalledTimes(1);
  });
});
