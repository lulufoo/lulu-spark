// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

const apiMocks = vi.hoisted(() => ({
  searchSpark: vi.fn(),
  searchKnowledge: vi.fn(),
  fetchFileContent: vi.fn(),
}));

vi.mock('../../frontend/src/host/api.ts', () => ({
  searchSpark: (...args) => apiMocks.searchSpark(...args),
  searchKnowledge: (...args) => apiMocks.searchKnowledge(...args),
  fetchFileContent: (...args) => apiMocks.fetchFileContent(...args),
}));

function seedSparkSearchDom() {
  document.body.innerHTML = `
    <div id="gs-wb-wrap" class="gs-search-wrap">
      <input id="gs-wb-input" class="gs-search-input" type="text" autocomplete="off" />
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
  return import('../../frontend/src/notes/ui/search.tsx');
}

describe('notes search module', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    installLocalStorageMock();
    seedSparkSearchDom();
    apiMocks.searchSpark.mockResolvedValue({ hits: [] });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('exports initSparkSearch and closeSparkSearch', async () => {
    const mod = await loadModule();
    expect(typeof mod.initSparkSearch).toBe('function');
    expect(typeof mod.closeSparkSearch).toBe('function');
  });

  it('debounced input calls searchSpark(q, 8) only', async () => {
    const { initSparkSearch } = await loadModule();
    initSparkSearch();

    const input = document.getElementById('gs-wb-input');
    input.value = 'alpha';
    input.dispatchEvent(new Event('input', { bubbles: true }));

    await vi.advanceTimersByTimeAsync(300);

    expect(apiMocks.searchSpark).toHaveBeenCalledWith('alpha', 8);
    expect(apiMocks.searchKnowledge).not.toHaveBeenCalled();
  });

  it('strips leading # before calling searchSpark', async () => {
    const { initSparkSearch } = await loadModule();
    initSparkSearch();

    const input = document.getElementById('gs-wb-input');
    input.value = '#topic-name';
    input.dispatchEvent(new Event('input', { bubbles: true }));

    await vi.advanceTimersByTimeAsync(300);

    expect(apiMocks.searchSpark).toHaveBeenCalledWith('topic-name', 8);
  });

  it('hit click dispatches cta:open-entry with common_path only (no layer)', async () => {
    apiMocks.searchSpark.mockResolvedValue({
      hits: [{
        title: 'Entry',
        common_path: '2024/01/entry.md',
        layer: 'raw',
        topic: 'demo',
        body: 'snippet body',
      }],
    });

    const { initSparkSearch } = await loadModule();
    initSparkSearch();

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
    });
    expect('layer' in handler.mock.calls[0][0].detail).toBe(false);
  });

  it('renders no per-field rebuild button (header owns index rebuild)', async () => {
    const { initSparkSearch } = await loadModule();
    initSparkSearch();
    expect(document.getElementById('gs-wb-rebuild-btn')).toBeNull();
    expect(document.querySelector('#gs-wb-wrap .gs-rebuild-btn')).toBeNull();
  });

  it('stores history under gs-history-wb without # prefix', async () => {
    apiMocks.searchSpark.mockResolvedValue({
      hits: [{
        title: 'Hit',
        common_path: 'path/a.md',
        layer: 'digest',
        topic: 't',
        body: 'body',
      }],
    });

    const { initSparkSearch } = await loadModule();
    initSparkSearch();

    const input = document.getElementById('gs-wb-input');
    input.value = '#stored-query';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    document.querySelector('.gs-hit-wb').click();

    const stored = JSON.parse(localStorage.getItem('gs-history-wb') || '[]');
    expect(stored).toContain('stored-query');
    expect(stored.some((q) => q.startsWith('#'))).toBe(false);
  });

  it('shows search error for unexpected backend failure', async () => {
    apiMocks.searchSpark.mockResolvedValue({ error: 'unavailable' });

    const { initSparkSearch } = await loadModule();
    initSparkSearch();

    document.getElementById('gs-wb-input').value = 'q';
    document.getElementById('gs-wb-input').dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    const dropdown = document.getElementById('gs-wb-dropdown');
    expect(dropdown.style.display).toBe('block');
    expect(dropdown.textContent).toContain('Search error');
  });

  it('shows global-search-equivalent status for not_indexed', async () => {
    apiMocks.searchSpark.mockResolvedValue({ error: 'not_indexed' });

    const { initSparkSearch } = await loadModule();
    initSparkSearch();

    document.getElementById('gs-wb-input').value = 'q';
    document.getElementById('gs-wb-input').dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    const dropdown = document.getElementById('gs-wb-dropdown');
    expect(dropdown.textContent).toContain('Index not built yet.');
    expect(dropdown.textContent).not.toContain('header');
  });

  it('shows no-results and search-error statuses', async () => {
    const { initSparkSearch } = await loadModule();
    initSparkSearch();

    document.getElementById('gs-wb-input').value = 'empty';
    document.getElementById('gs-wb-input').dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);
    expect(document.getElementById('gs-wb-dropdown').textContent).toContain('No related results');

    apiMocks.searchSpark.mockRejectedValue(new Error('network'));
    document.getElementById('gs-wb-input').value = 'err';
    document.getElementById('gs-wb-input').dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);
    expect(document.getElementById('gs-wb-dropdown').textContent).toContain('Search error');
  });

  it('Escape closes dropdown and blurs input', async () => {
    const { initSparkSearch } = await loadModule();
    initSparkSearch();

    const input = document.getElementById('gs-wb-input');
    const dropdown = document.getElementById('gs-wb-dropdown');
    dropdown.style.display = 'block';

    input.focus();
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));

    expect(dropdown.style.display).toBe('none');
    expect(document.activeElement).not.toBe(input);
  });

  it('document click outside #gs-wb-wrap closes dropdown', async () => {
    const { initSparkSearch } = await loadModule();
    initSparkSearch();

    const dropdown = document.getElementById('gs-wb-dropdown');
    dropdown.style.display = 'block';

    document.getElementById('outside-click-target').click();

    expect(dropdown.style.display).toBe('none');
  });

  it('closeSparkSearch hides #gs-wb-dropdown', async () => {
    const { closeSparkSearch } = await loadModule();
    const dropdown = document.getElementById('gs-wb-dropdown');
    dropdown.style.display = 'block';

    closeSparkSearch();

    expect(dropdown.style.display).toBe('none');
  });

  it('initSparkSearch is idempotent (no duplicate search on one input)', async () => {
    const { initSparkSearch } = await loadModule();
    initSparkSearch();
    initSparkSearch();

    const input = document.getElementById('gs-wb-input');
    input.value = 'once';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    expect(apiMocks.searchSpark).toHaveBeenCalledTimes(1);
  });

  it('hit row hover does not show digest tooltip or fetch digest', async () => {
    apiMocks.searchSpark.mockResolvedValue({
      hits: [{ title: 'Tip', common_path: 'inbox/notes/tip.md', topic: 't', body: 'b' }],
    });
    apiMocks.fetchFileContent.mockResolvedValueOnce('# Tip **preview**');

    const { initSparkSearch } = await loadModule();
    initSparkSearch();

    const input = document.getElementById('gs-wb-input');
    input.value = 'tip';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    const hit = document.querySelector('.gs-hit-wb');
    expect(hit).toBeTruthy();

    hit.dispatchEvent(new MouseEvent('mouseenter'));
    await vi.advanceTimersByTimeAsync(300);
    expect(apiMocks.fetchFileContent).not.toHaveBeenCalled();
    expect(document.querySelector('.digest-tooltip')).toBeNull();
  });

  it('search.tsx does not import attachDigestTooltip', () => {
    const src = readFileSync(join(repoRoot, 'frontend/src/notes/ui/search.tsx'), 'utf8');
    expect(src).not.toMatch(/attachDigestTooltip/);
    expect(src).not.toMatch(/digest-tooltip/);
  });
});
