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

vi.mock('../../frontend/src/knowledge/ui/search.tsx', () => ({
  closeKnowledgeSearch: vi.fn(),
  initKnowledgeSearch: vi.fn(),
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

function seedWorkbenchSearchDom() {
  document.body.innerHTML = `
    <button id="btn-nav-home-title" hidden></button>
    <button id="btn-nav-home"></button>
    <div id="gs-wb-wrap" class="gs-search-wrap" hidden>
      <input id="gs-wb-input" class="gs-search-input" type="text" autocomplete="off" />
      <div id="gs-wb-dropdown" class="gs-search-dropdown" style="display:none"></div>
    </div>
    <div id="gs-kb-wrap" class="gs-search-wrap" hidden>
      <input id="gs-kb-input" class="gs-search-input" type="text" />
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

/** Mirrors main.js wrapRouteMount + workbench mount contract under test. */
async function simulateWorkbenchRouteMount() {
  applySearchNavChrome('workbench');
  const { initWorkbenchSearch } = await import('../../frontend/src/notes/ui/search.tsx');
  initWorkbenchSearch();
}

async function loadWorkbenchSearchModule() {
  vi.resetModules();
  return import('../../frontend/src/notes/ui/search.tsx');
}

describe('main.js workbench route init wiring (source)', () => {
  it('imports initWorkbenchSearch from notes/ui/search.tsx', () => {
    expect(mainJs).toMatch(
      /import\s*\{[^}]*initWorkbenchSearch[^}]*\}\s*from\s*'[^']*notes\/ui\/search\.tsx'/,
    );
  });

  it('calls initWorkbenchSearch inside wrapRouteMount for workbench route mount', () => {
    const wrapBody = extractFunctionBody(mainJs, 'wrapRouteMount');
    expect(wrapBody).toMatch(/initWorkbenchSearch\s*\(\s*\)/);
    expect(wrapBody).toMatch(/routeName\s*===\s*['"]workbench['"]/);
  });

  it('registers workbench handler via wrapRouteMount', () => {
    expect(mainJs).toMatch(/workbench:\s*wrapRouteMount\s*\(\s*['"]workbench['"]/);
  });
});

describe('workbench route mount behavior', () => {
  beforeEach(async () => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    installLocalStorageMock();
    seedWorkbenchSearchDom();
    apiMocks.searchWorkbench.mockResolvedValue({ hits: [] });
    await loadWorkbenchSearchModule();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('workbench mount: wb wrap visible and input enabled after nav chrome', async () => {
    await simulateWorkbenchRouteMount();

    expect(document.getElementById('gs-wb-wrap').hidden).toBe(false);
    expect(document.getElementById('gs-kb-wrap').hidden).toBe(true);
    expect(document.getElementById('gs-wb-input').disabled).toBe(false);
  });

  it('leaving workbench route calls closeWorkbenchSearch', async () => {
    await simulateWorkbenchRouteMount();

    const dropdown = document.getElementById('gs-wb-dropdown');
    dropdown.style.display = 'block';
    applySearchNavChrome('home');

    expect(dropdown.style.display).toBe('none');
  });

  it('initWorkbenchSearch is idempotent on repeated workbench route mounts', async () => {
    await simulateWorkbenchRouteMount();
    await simulateWorkbenchRouteMount();

    const input = document.getElementById('gs-wb-input');
    input.value = 'alpha';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    expect(apiMocks.searchWorkbench).toHaveBeenCalledTimes(1);
    expect(apiMocks.searchKnowledge).not.toHaveBeenCalled();
  });

  it('TAC-2: workbench search input triggers searchWorkbench only', async () => {
    await simulateWorkbenchRouteMount();

    const input = document.getElementById('gs-wb-input');
    input.value = 'topic';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    expect(apiMocks.searchWorkbench).toHaveBeenCalledWith('topic', 8);
    expect(apiMocks.searchKnowledge).not.toHaveBeenCalled();
  });
});

function extractFunctionSource(source, name) {
  const start = source.indexOf(`function ${name}`);
  if (start === -1) return '';
  const braceStart = source.indexOf('{', start);
  let depth = 0;
  for (let i = braceStart; i < source.length; i += 1) {
    if (source[i] === '{') depth += 1;
    else if (source[i] === '}') {
      depth -= 1;
      if (depth === 0) return source.slice(start, i + 1);
    }
  }
  return '';
}

function seedWorkbenchMountDom() {
  document.body.innerHTML = `
    <div id="home-view" style="display:none"></div>
    <div id="knowledge-doc-view" style="display:none"></div>
    <div id="read-later-view" style="display:none"></div>
    <div id="todo-tasks-view" style="display:none"></div>
    <div class="layout">
      <main id="main">
        <div id="status"></div>
        <div id="date-heading" style="display:none"></div>
        <div id="doc-list"></div>
        <div id="feed-view" style="display:none"></div>
      </main>
    </div>
    <div id="md-modal" class="viewer-modal" style="display:none"></div>
  `;
}

function compileMountWorkbench(env) {
  const fnSource = extractFunctionSource(mainJs, 'mountWorkbench');
  expect(fnSource, 'mountWorkbench missing').not.toBe('');
  const locals = [
    'clearHeaderSyncKnowledgeContext',
    'hideKnowledgeDocView',
    'hideReadLaterView',
    'hideTodoTasksView',
    'hideHomeView',
    'feedView',
    'state',
    'selectDate',
    'openDoc',
    'notifyState',
  ];
  const brace = fnSource.indexOf('{');
  const body = fnSource.slice(brace + 1, fnSource.lastIndexOf('}'));
  const prelude = locals.map((k) => `var ${k} = __env.${k};`).join('\n');
  // eslint-disable-next-line no-new-func
  return new Function(
    '__env',
    'document',
    `${prelude}\nreturn function (route) {\n${body}\n};`,
  )(env, document);
}

function stubWorkbenchMountEnv(overrides = {}) {
  const selectDate = vi.fn();
  const openDoc = vi.fn(async () => {});
  const state = {
    ui: { activeDate: null, ...(overrides.state?.ui || {}) },
    index: { data: null, ...(overrides.state?.index || {}) },
    viewer: { createSession: null, ...(overrides.state?.viewer || {}) },
  };
  return {
    clearHeaderSyncKnowledgeContext: () => {},
    hideKnowledgeDocView: () => {},
    hideReadLaterView: () => {},
    hideTodoTasksView: () => {},
    hideHomeView: () => {},
    feedView: document.getElementById('feed-view') || { style: { display: '' } },
    selectDate,
    openDoc,
    notifyState: vi.fn(),
    state,
  };
}

describe('T2 mountWorkbench three-branch routing', () => {
  beforeEach(() => {
    seedWorkbenchMountDom();
  });

  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('passes route into mountWorkbench (does not discard params)', () => {
    expect(mainJs).toMatch(
      /workbench:\s*wrapRouteMount\s*\(\s*['"]workbench['"]\s*,\s*(?:\(\s*route\s*\)\s*=>\s*mountWorkbench\s*\(\s*route\s*\)|mountWorkbench)\s*\)/,
    );
  });

  it('resolved note → note outlet open state (not Dialog)', () => {
    const entry = { common_path: 'inbox/notes/a.md', layers: ['raw'] };
    const env = stubWorkbenchMountEnv({
      state: { index: { data: { e1: entry } } },
    });
    const mount = compileMountWorkbench(env);

    mount({ params: { date: '20260719', note: 'inbox/notes/a.md', layer: 'raw' } });

    expect(env.openDoc).toHaveBeenCalledWith(entry, 'raw');
    expect(env.state.viewer.outletMode).toBe('open');
    expect(env.selectDate).not.toHaveBeenCalled();
  });

  it('unresolved note → safe empty with visible prompt; keeps note; no Dialog', () => {
    const env = stubWorkbenchMountEnv({
      state: { index: { data: {} } },
    });
    const mount = compileMountWorkbench(env);
    const hashBefore = '#/workbench?date=20260719&note=missing/path.md';
    window.location.hash = hashBefore;

    mount({ params: { date: '20260719', note: 'missing/path.md' } });

    expect(env.openDoc).not.toHaveBeenCalled();
    expect(env.state.viewer.outletMode).toBe('safe-empty');
    expect(env.state.viewer.outletMessage).toMatch(/Note not found/);
    expect(window.location.hash).toBe(hashBefore);
    expect(env.selectDate).not.toHaveBeenCalled();
  });

  it('no note + create in progress → note outlet create state (not list)', () => {
    const env = stubWorkbenchMountEnv({
      state: {
        viewer: { createSession: { tempId: 'tmp-1', status: 'creating' } },
      },
    });
    const mount = compileMountWorkbench(env);

    mount({ params: { date: '20260719' } });

    expect(env.state.viewer.outletMode).toBe('create');
    expect(env.selectDate).not.toHaveBeenCalled();
    expect(env.openDoc).not.toHaveBeenCalled();
  });

  it('no note + not creating → list via selectDate from location.date', () => {
    const env = stubWorkbenchMountEnv();
    const mount = compileMountWorkbench(env);

    mount({ params: { date: '20260719' } });

    expect(env.selectDate).toHaveBeenCalledWith('20260719');
    expect(env.state.viewer.outletMode).toBe('');
    expect(env.openDoc).not.toHaveBeenCalled();
  });

  it('forbids treating missing note as always-list when createSession is active', () => {
    const env = stubWorkbenchMountEnv({
      state: {
        viewer: { createSession: { tempId: 'tmp-2', status: 'creating' } },
      },
    });
    const mount = compileMountWorkbench(env);

    mount({ params: {} });

    expect(env.selectDate).not.toHaveBeenCalled();
    expect(env.state.viewer.outletMode).toBe('create');
  });
});
