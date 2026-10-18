// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
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

describe('main.js spark route wiring (source)', () => {
  it('no longer initialises an inline search on spark mount (the search is a modal owned by the notes page)', () => {
    expect(mainJs).not.toMatch(/initSparkSearch/);
    expect(extractFunctionBody(mainJs, 'wrapRouteMount')).not.toMatch(/Search/);
  });

  it('registers spark handler via wrapRouteMount', () => {
    expect(mainJs).toMatch(/spark:\s*wrapRouteMount\s*\(\s*['"]spark['"]/);
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

function seedSparkMountDom() {
  document.body.innerHTML = `
    <div id="home-view" style="display:none"></div>
    <div id="knowledge-doc-view" style="display:none"></div>
    <div id="read-later-view" style="display:none"></div>
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

function compileMountSpark(env) {
  const fnSource = extractFunctionSource(mainJs, 'mountSpark');
  expect(fnSource, 'mountSpark missing').not.toBe('');
  const locals = [
    'hideKnowledgeDocView',
    'hideReadLaterView',
    'hideHomeView',
    'feedView',
    'state',
    'selectDate',
    'openDoc',
    'notifyState',
    'logNotifyHop',
    'parseTraceId',
    'lastNotifyTrace',
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

function stubSparkMountEnv(overrides = {}) {
  const selectDate = vi.fn();
  const openDoc = vi.fn(async () => {});
  const state = {
    ui: { activeDate: null, ...(overrides.state?.ui || {}) },
    index: { data: null, ...(overrides.state?.index || {}) },
    viewer: { createSession: null, ...(overrides.state?.viewer || {}) },
  };
  return {
    hideKnowledgeDocView: () => {},
    hideReadLaterView: () => {},
    hideHomeView: () => {},
    feedView: document.getElementById('feed-view') || { style: { display: '' } },
    selectDate,
    openDoc,
    notifyState: vi.fn(),
    logNotifyHop: vi.fn(),
    parseTraceId: vi.fn(),
    lastNotifyTrace: vi.fn(),
    state,
  };
}

describe('T2 mountSpark three-branch routing', () => {
  beforeEach(() => {
    seedSparkMountDom();
  });

  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('passes route into mountSpark (does not discard params)', () => {
    expect(mainJs).toMatch(
      /spark:\s*wrapRouteMount\s*\(\s*['"]spark['"]\s*,\s*(?:\(\s*route\s*\)\s*=>\s*mountSpark\s*\(\s*route\s*\)|mountSpark)\s*\)/,
    );
  });

  it('resolved note → note outlet open state (not Dialog)', () => {
    const entry = { id: 'e1', project_id: 'inbox', layers: ['raw'] };
    const env = stubSparkMountEnv({
      state: { index: { data: { e1: entry } } },
    });
    const mount = compileMountSpark(env);

    mount({ params: { date: '20260719', note: 'e1', layer: 'raw' } });

    expect(env.openDoc).toHaveBeenCalledWith(entry, 'raw');
    expect(env.state.viewer.outletMode).toBe('open');
    expect(env.selectDate).not.toHaveBeenCalled();
  });

  it('unresolved note → safe empty with visible prompt; keeps note; no Dialog', () => {
    const env = stubSparkMountEnv({
      state: { index: { data: {} } },
    });
    const mount = compileMountSpark(env);
    const hashBefore = '#/spark?date=20260719&note=missing/path.md';
    window.location.hash = hashBefore;

    mount({ params: { date: '20260719', note: 'missing/path.md' } });

    expect(env.openDoc).not.toHaveBeenCalled();
    expect(env.state.viewer.outletMode).toBe('safe-empty');
    expect(env.state.viewer.outletMessage).toMatch(/Note not found/);
    expect(window.location.hash).toBe(hashBefore);
    expect(env.selectDate).not.toHaveBeenCalled();
  });

  it('no note + create in progress → note outlet create state (not list)', () => {
    const env = stubSparkMountEnv({
      state: {
        viewer: { createSession: { tempId: 'tmp-1', status: 'creating' } },
      },
    });
    const mount = compileMountSpark(env);

    mount({ params: { date: '20260719' } });

    expect(env.state.viewer.outletMode).toBe('create');
    expect(env.selectDate).not.toHaveBeenCalled();
    expect(env.openDoc).not.toHaveBeenCalled();
  });

  it('no note + not creating → list via selectDate from location.date', () => {
    const env = stubSparkMountEnv();
    const mount = compileMountSpark(env);

    mount({ params: { date: '20260719' } });

    expect(env.selectDate).toHaveBeenCalledWith('20260719');
    expect(env.state.viewer.outletMode).toBe('');
    expect(env.openDoc).not.toHaveBeenCalled();
  });

  it('forbids treating missing note as always-list when createSession is active', () => {
    const env = stubSparkMountEnv({
      state: {
        viewer: { createSession: { tempId: 'tmp-2', status: 'creating' } },
      },
    });
    const mount = compileMountSpark(env);

    mount({ params: {} });

    expect(env.selectDate).not.toHaveBeenCalled();
    expect(env.state.viewer.outletMode).toBe('create');
  });
});
