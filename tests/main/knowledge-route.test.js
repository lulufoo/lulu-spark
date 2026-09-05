// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const apiMocks = vi.hoisted(() => ({
  searchWorkbench: vi.fn(),
  searchKnowledge: vi.fn(),
}));

vi.mock('../../frontend/src/host/api.ts', () => ({
  searchWorkbench: (...args) => apiMocks.searchWorkbench(...args),
  searchKnowledge: (...args) => apiMocks.searchKnowledge(...args),
}));

vi.mock('../../frontend/src/notes/ui/search.tsx', () => ({
  closeWorkbenchSearch: vi.fn(),
  initWorkbenchSearch: vi.fn(),
}));

import { applySearchNavChrome } from '../../frontend/src/app-shell/ui/nav-chrome.ts';
import { readMainSource } from '../helpers/read-frontend-js.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const mainJs = readMainSource();

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

function seedKnowledgeRouteDom() {
  document.body.innerHTML = `
    <button id="btn-nav-home-title" hidden></button>
    <button id="btn-nav-home"></button>
    <div id="gs-wb-wrap" class="gs-search-wrap" hidden>
      <input id="gs-wb-input" class="gs-search-input" type="text" autocomplete="off" />
    </div>
    <div id="gs-kb-wrap" class="gs-search-wrap" hidden>
      <input id="gs-kb-input" class="gs-search-input" type="text" autocomplete="off" />
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

/** Mirrors main.js wrapRouteMount + knowledge-doc mount contract under test. */
async function simulateKnowledgeRouteMount() {
  applySearchNavChrome('knowledge-doc');
  const { initKnowledgeSearch } = await import('../../frontend/src/knowledge/ui/search.tsx');
  initKnowledgeSearch();
}

async function loadKnowledgeSearchModule() {
  vi.resetModules();
  return import('../../frontend/src/knowledge/ui/search.tsx');
}

describe('main.js knowledge-doc route init wiring (source)', () => {
  it('imports initKnowledgeSearch from knowledge/ui/search.tsx', () => {
    expect(mainJs).toMatch(
      /import\s*\{[^}]*initKnowledgeSearch[^}]*\}\s*from\s*'[^']*knowledge\/ui\/search\.tsx'/,
    );
  });

  it('calls initKnowledgeSearch inside wrapRouteMount for knowledge-doc route mount', () => {
    const wrapBody = extractFunctionBody(mainJs, 'wrapRouteMount');
    expect(wrapBody).toMatch(/initKnowledgeSearch\s*\(\s*\)/);
    expect(wrapBody).toMatch(/routeName\s*===\s*['"]knowledge-doc['"]/);
  });

  it('registers knowledge-doc handler via wrapRouteMount', () => {
    expect(mainJs).toMatch(/['"]knowledge-doc['"]:\s*wrapRouteMount\s*\(\s*['"]knowledge-doc['"]/);
  });

  it('cta:open-kb-doc navigates to knowledge deep link instead of openKbDoc', () => {
    expect(mainJs).toMatch(/document\.addEventListener\(\s*['"]cta:open-kb-doc['"]/);
    expect(mainJs).toMatch(
      /navigate\s*\(\s*['"`]#\/knowledge\/['"`]\s*\+\s*encodeURIComponent\(detail\.repo\)\s*\+\s*['"`]\?path=['"`]\s*\+\s*encodeURIComponent\(detail\.path\)\s*\)/,
    );
    expect(mainJs).not.toMatch(/openKbDoc\s*\(\s*detail\s*\)/);
  });
});

describe('knowledge-doc route mount behavior', () => {
  beforeEach(async () => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    installLocalStorageMock();
    seedKnowledgeRouteDom();
    apiMocks.searchKnowledge.mockResolvedValue({ hits: [] });
    await loadKnowledgeSearchModule();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('knowledge-doc mount: kb wrap visible and input enabled after nav chrome', async () => {
    await simulateKnowledgeRouteMount();

    expect(document.getElementById('gs-wb-wrap').hidden).toBe(true);
    expect(document.getElementById('gs-kb-wrap').hidden).toBe(false);
    expect(document.getElementById('gs-kb-input').disabled).toBe(false);
  });

  it('leaving knowledge-doc route calls closeKnowledgeSearch', async () => {
    await simulateKnowledgeRouteMount();

    const dropdown = document.getElementById('gs-kb-dropdown');
    dropdown.style.display = 'block';
    applySearchNavChrome('home');

    expect(dropdown.style.display).toBe('none');
  });

  it('initKnowledgeSearch is idempotent on repeated knowledge-doc route mounts', async () => {
    await simulateKnowledgeRouteMount();
    await simulateKnowledgeRouteMount();

    const input = document.getElementById('gs-kb-input');
    input.value = 'alpha';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    expect(apiMocks.searchKnowledge).toHaveBeenCalledTimes(1);
    expect(apiMocks.searchWorkbench).not.toHaveBeenCalled();
  });

  it('TAC-3: knowledge search input triggers searchKnowledge only', async () => {
    await simulateKnowledgeRouteMount();

    const input = document.getElementById('gs-kb-input');
    input.value = 'topic';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    expect(apiMocks.searchKnowledge).toHaveBeenCalledWith('topic', 8);
    expect(apiMocks.searchWorkbench).not.toHaveBeenCalled();
  });
});
