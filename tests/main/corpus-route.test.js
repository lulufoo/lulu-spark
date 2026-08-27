// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const apiMocks = vi.hoisted(() => ({
  searchWorkbench: vi.fn(),
  searchKnowledge: vi.fn(),
  reindexKnowledge: vi.fn(),
  getReindexStatus: vi.fn(),
}));

vi.mock('../../frontend/js/host/api.js', () => ({
  searchWorkbench: (...args) => apiMocks.searchWorkbench(...args),
  searchKnowledge: (...args) => apiMocks.searchKnowledge(...args),
  reindexKnowledge: (...args) => apiMocks.reindexKnowledge(...args),
  getReindexStatus: (...args) => apiMocks.getReindexStatus(...args),
}));

vi.mock('../../frontend/js/notes/search.js', () => ({
  closeWorkbenchSearch: vi.fn(),
  initWorkbenchSearch: vi.fn(),
}));

import { applySearchNavChrome } from '../../frontend/js/app-shell/nav-chrome.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const mainJs = readFileSync(join(fixtureRoot, 'frontend/js/main.js'), 'utf8');

function extractFunctionBody(source, name) {
  const start = source.indexOf(`function ${name}`);
  if (start === -1) return '';
  const braceStart = source.indexOf('{', start);
  let depth = 0;
  for (let i = braceStart; i < source.length; i += 1) {
    if (source[i] === '{') depth += 1;
    if (source[i] === '}') {
      depth -= 1;
      if (depth === 0) return source.slice(braceStart, i + 1);
    }
  }
  return '';
}

function seedCorpusRouteDom() {
  document.body.innerHTML = `
    <button id="btn-nav-home-title" hidden></button>
    <button id="btn-nav-home"></button>
    <div id="gs-wb-wrap" class="gs-search-wrap" hidden>
      <input id="gs-wb-input" class="gs-search-input" type="text" autocomplete="off" />
    </div>
    <div id="gs-kb-wrap" class="gs-search-wrap" hidden>
      <input id="gs-kb-input" class="gs-search-input" type="text" autocomplete="off" />
      <button id="gs-kb-rebuild-btn" class="gs-rebuild-btn" style="display:none"></button>
      <div id="gs-kb-dropdown" class="gs-search-dropdown" style="display:none"></div>
    </div>
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

/** Mirrors main.js wrapRouteMount + corpus-doc mount contract under test. */
async function simulateCorpusRouteMount() {
  applySearchNavChrome('corpus-doc');
  const { initCorpusSearch } = await import('../../frontend/js/corpus/corpus-search.js');
  initCorpusSearch();
}

async function loadCorpusSearchModule() {
  vi.resetModules();
  return import('../../frontend/js/corpus/corpus-search.js');
}

describe('main.js corpus-doc route init wiring (source)', () => {
  it('imports initCorpusSearch from corpus-search.js', () => {
    expect(mainJs).toMatch(
      /import\s*\{[^}]*initCorpusSearch[^}]*\}\s*from\s*'\.\/corpus\/corpus-search\.js'/,
    );
  });

  it('calls initCorpusSearch inside wrapRouteMount for corpus-doc route mount', () => {
    const wrapBody = extractFunctionBody(mainJs, 'wrapRouteMount');
    expect(wrapBody).toMatch(/initCorpusSearch\s*\(\s*\)/);
    expect(wrapBody).toMatch(/routeName\s*===\s*['"]corpus-doc['"]/);
  });

  it('registers corpus-doc handler via wrapRouteMount', () => {
    expect(mainJs).toMatch(/['"]corpus-doc['"]:\s*wrapRouteMount\s*\(\s*['"]corpus-doc['"]/);
  });

  it('cta:open-kb-doc navigates to corpus deep link instead of openKbDoc', () => {
    expect(mainJs).toMatch(/document\.addEventListener\(\s*['"]cta:open-kb-doc['"]/);
    expect(mainJs).toMatch(
      /navigate\s*\(\s*['"`]#\/corpus\/['"`]\s*\+\s*encodeURIComponent\(detail\.repo\)\s*\+\s*['"`]\?path=['"`]\s*\+\s*encodeURIComponent\(detail\.path\)\s*\)/,
    );
    expect(mainJs).not.toMatch(/openKbDoc\s*\(\s*detail\s*\)/);
  });
});

describe('corpus-doc route mount behavior', () => {
  beforeEach(async () => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    installLocalStorageMock();
    seedCorpusRouteDom();
    apiMocks.searchKnowledge.mockResolvedValue({ hits: [] });
    await loadCorpusSearchModule();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('corpus-doc mount: kb wrap visible and input enabled after nav chrome', async () => {
    await simulateCorpusRouteMount();

    expect(document.getElementById('gs-wb-wrap').hidden).toBe(true);
    expect(document.getElementById('gs-kb-wrap').hidden).toBe(false);
    expect(document.getElementById('gs-kb-input').disabled).toBe(false);
  });

  it('leaving corpus-doc route calls closeCorpusSearch', async () => {
    await simulateCorpusRouteMount();

    const dropdown = document.getElementById('gs-kb-dropdown');
    dropdown.style.display = 'block';
    applySearchNavChrome('home');

    expect(dropdown.style.display).toBe('none');
  });

  it('initCorpusSearch is idempotent on repeated corpus-doc route mounts', async () => {
    await simulateCorpusRouteMount();
    await simulateCorpusRouteMount();

    const input = document.getElementById('gs-kb-input');
    input.value = 'alpha';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    expect(apiMocks.searchKnowledge).toHaveBeenCalledTimes(1);
    expect(apiMocks.searchWorkbench).not.toHaveBeenCalled();
  });

  it('TAC-3: corpus search input triggers searchKnowledge only', async () => {
    await simulateCorpusRouteMount();

    const input = document.getElementById('gs-kb-input');
    input.value = 'topic';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    expect(apiMocks.searchKnowledge).toHaveBeenCalledWith('topic', 8);
    expect(apiMocks.searchWorkbench).not.toHaveBeenCalled();
  });
});
