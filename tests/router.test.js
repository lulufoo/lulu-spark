import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import {
  parseHash,
  initRouter,
  navigate,
  normalizeHash,
  navigateToNote,
  navigateBackToList,
  navigateToDateList,
} from '../frontend/js/router/index.js';

describe('parseHash', () => {
  it('parses #/workbench', () => {
    expect(parseHash('#/workbench')).toEqual({ name: 'workbench', params: {} });
  });

  it('parses #/workbench?date=&note=&layer= query params', () => {
    expect(
      parseHash('#/workbench?date=20260719&note=inbox/notes/x.md&layer=raw'),
    ).toEqual({
      name: 'workbench',
      params: {
        date: '20260719',
        note: 'inbox/notes/x.md',
        layer: 'raw',
      },
    });
  });

  it('parses #/workbench with only date query', () => {
    expect(parseHash('#/workbench?date=20260719')).toEqual({
      name: 'workbench',
      params: { date: '20260719' },
    });
  });

  it('parses #/workbench with date and note but no layer', () => {
    const result = parseHash('#/workbench?date=20260719&note=inbox/notes/x.md');
    expect(result).toEqual({
      name: 'workbench',
      params: {
        date: '20260719',
        note: 'inbox/notes/x.md',
      },
    });
    expect(result.params).not.toHaveProperty('layer');
  });

  it('does not fabricate note id when note and layer are absent', () => {
    const result = parseHash('#/workbench?date=20260719');
    expect(result.name).toBe('workbench');
    expect(result.params).not.toHaveProperty('note');
    expect(result.params).not.toHaveProperty('layer');
  });

  it('parses #/corpus as empty repo corpus-doc', () => {
    expect(parseHash('#/corpus')).toEqual({ name: 'corpus-doc', params: { repo: '' } });
  });

  it('parses legacy #/corpus/pick as empty repo corpus-doc', () => {
    expect(parseHash('#/corpus/pick')).toEqual({ name: 'corpus-doc', params: { repo: '' } });
  });

  it('parses #/corpus/:repo with slash in repo', () => {
    expect(parseHash('#/corpus/lulufoo/myrepo')).toEqual({
      name: 'corpus-doc',
      params: { repo: 'lulufoo/myrepo' },
    });
    expect(parseHash('#/corpus/lulufoo%2Fmyrepo')).toEqual({
      name: 'corpus-doc',
      params: { repo: 'lulufoo/myrepo' },
    });
  });

  it('parses #/home', () => {
    expect(parseHash('#/home')).toEqual({ name: 'home', params: {} });
  });

  it('parses #/read-later', () => {
    expect(parseHash('#/read-later')).toEqual({ name: 'read-later', params: {} });
  });

  it('parses #/read-later/ with trailing slash', () => {
    expect(parseHash('#/read-later/')).toEqual({ name: 'read-later', params: {} });
  });

  it('parses #/corpus/:repo?path= for deep link', () => {
    expect(parseHash('#/corpus/owner/repo?path=docs/guide.md')).toEqual({
      name: 'corpus-doc',
      params: { repo: 'owner/repo', path: 'docs/guide.md' },
    });
  });

  it('decodes repo and path when encoded', () => {
    expect(parseHash('#/corpus/lulufoo%2Fmyrepo?path=readme.md')).toEqual({
      name: 'corpus-doc',
      params: { repo: 'lulufoo/myrepo', path: 'readme.md' },
    });
  });

  it('omits path when ?path= query is absent', () => {
    const result = parseHash('#/corpus/lulufoo/myrepo');
    expect(result).toEqual({
      name: 'corpus-doc',
      params: { repo: 'lulufoo/myrepo' },
    });
    expect(result.params).not.toHaveProperty('path');
  });

  it('handles empty ?path= value', () => {
    expect(parseHash('#/corpus/owner/repo?path=')).toEqual({
      name: 'corpus-doc',
      params: { repo: 'owner/repo', path: '' },
    });
  });

  it('does not merge query into repo when repo contains slash', () => {
    expect(parseHash('#/corpus/lulufoo/myrepo?path=a.md')).toEqual({
      name: 'corpus-doc',
      params: { repo: 'lulufoo/myrepo', path: 'a.md' },
    });
  });

  it('returns unknown for invalid repo encoding with query', () => {
    expect(parseHash('#/corpus/%?path=foo.md')).toEqual({
      name: 'unknown',
      params: {},
    });
  });
});

describe('initRouter fallback', () => {
  let handlers;
  let hashValue;
  let listeners;

  beforeEach(() => {
    handlers = {
      workbench: vi.fn(),
      home: vi.fn(),
      'corpus-doc': vi.fn(),
    };
    hashValue = '';
    listeners = {};
    vi.stubGlobal('window', {
      addEventListener(type, fn) {
        listeners[type] = fn;
      },
      location: {
        get hash() {
          return hashValue;
        },
        set hash(value) {
          hashValue = value;
          listeners.hashchange?.();
        },
        replace(value) {
          const idx = value.indexOf('#');
          hashValue = idx >= 0 ? value.slice(idx) : value;
          listeners.hashchange?.();
        },
      },
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('empty hash falls back to #/workbench', () => {
    hashValue = '';
    initRouter(handlers, { fallback: '#/workbench' });
    expect(hashValue).toBe('#/workbench');
    expect(handlers.workbench).toHaveBeenCalledTimes(1);
  });

  it('unknown hash falls back to #/workbench', () => {
    hashValue = '#/unknown';
    initRouter(handlers, { fallback: '#/workbench' });
    expect(hashValue).toBe('#/workbench');
    expect(handlers.workbench).toHaveBeenCalledTimes(1);
  });

  it.each(['', '#', '#/', '#/unknown'])(
    'initRouter redirects %s to fallback and mounts workbench handler',
    (hash) => {
      hashValue = hash;
      initRouter(handlers, { fallback: '#/workbench' });
      expect(hashValue).toBe('#/workbench');
      expect(handlers.workbench).toHaveBeenCalledTimes(1);
    },
  );
});

describe('hash navigation', () => {
  let handlers;
  let hashValue;
  let listeners;

  beforeEach(() => {
    hashValue = '#/workbench';
    listeners = {};
    handlers = {
      workbench: vi.fn(),
      home: vi.fn(),
      'corpus-doc': vi.fn(() => navigate('#/workbench')),
    };
    vi.stubGlobal('window', {
      addEventListener(type, fn) {
        listeners[type] = fn;
      },
      location: {
        get hash() {
          return hashValue;
        },
        set hash(value) {
          hashValue = value;
          listeners.hashchange?.();
        },
        replace(value) {
          const idx = value.indexOf('#');
          hashValue = idx >= 0 ? value.slice(idx) : value;
          listeners.hashchange?.();
        },
      },
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('popstate/hashchange re-invokes mount handler', () => {
    initRouter(handlers, { fallback: '#/workbench' });
    expect(handlers.workbench).toHaveBeenCalledTimes(1);
    handlers.workbench.mockClear();

    hashValue = '#/home';
    listeners.hashchange();
    expect(handlers.home).toHaveBeenCalledTimes(1);

    handlers.home.mockClear();
    hashValue = '#/corpus/owner/repo';
    listeners.popstate();
    expect(handlers['corpus-doc']).toHaveBeenCalledTimes(1);
  });

  it('back navigation from corpus route does not leave blank mount', () => {
    initRouter(handlers, { fallback: '#/workbench' });
    expect(handlers.workbench).toHaveBeenCalledTimes(1);

    hashValue = '#/corpus/owner/repo';
    listeners.hashchange();
    expect(handlers['corpus-doc']).toHaveBeenCalledTimes(1);

    const workbenchCallsAfterRedirect = handlers.workbench.mock.calls.length;
    expect(workbenchCallsAfterRedirect).toBeGreaterThanOrEqual(1);

    handlers.workbench.mockClear();
    hashValue = '#/workbench';
    listeners.popstate();

    expect(handlers.workbench).toHaveBeenCalledTimes(1);
    expect(() => handlers.workbench.mock.results[0]?.value).not.toThrow();
  });
});

describe('navigate', () => {
  let hashValue;
  let listeners;

  beforeEach(() => {
    hashValue = '#/workbench';
    listeners = {};
    vi.stubGlobal('window', {
      addEventListener(type, fn) {
        listeners[type] = fn;
      },
      location: {
        get hash() {
          return hashValue;
        },
        set hash(value) {
          hashValue = value;
          listeners.hashchange?.();
        },
      },
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('sets location.hash', () => {
    navigate('#/home');
    expect(hashValue).toBe('#/home');
  });
});

describe('Phase2 fallback (default #/home)', () => {
  let handlers;
  let hashValue;
  let listeners;

  beforeEach(() => {
    handlers = {
      workbench: vi.fn(),
      home: vi.fn(),
      'corpus-doc': vi.fn(),
    };
    hashValue = '';
    listeners = {};
    vi.stubGlobal('window', {
      addEventListener(type, fn) {
        listeners[type] = fn;
      },
      location: {
        get hash() {
          return hashValue;
        },
        set hash(value) {
          hashValue = value;
          listeners.hashchange?.();
        },
        replace(value) {
          const idx = value.indexOf('#');
          hashValue = idx >= 0 ? value.slice(idx) : value;
          listeners.hashchange?.();
        },
      },
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('empty hash falls back to #/home with default options', () => {
    hashValue = '';
    initRouter(handlers);
    expect(hashValue).toBe('#/home');
    expect(handlers.home).toHaveBeenCalledTimes(1);
  });

  it('unknown hash falls back to #/home with default options', () => {
    hashValue = '#/unknown';
    initRouter(handlers);
    expect(hashValue).toBe('#/home');
    expect(handlers.home).toHaveBeenCalledTimes(1);
  });

  it('does not replace #/workbench?… to #/home', () => {
    hashValue = '#/workbench?date=20260719&note=inbox/notes/x.md&layer=raw';
    initRouter(handlers);
    expect(hashValue).toBe('#/workbench?date=20260719&note=inbox/notes/x.md&layer=raw');
    expect(handlers.workbench).toHaveBeenCalledTimes(1);
    expect(handlers.workbench).toHaveBeenCalledWith({
      name: 'workbench',
      params: {
        date: '20260719',
        note: 'inbox/notes/x.md',
        layer: 'raw',
      },
    });
    expect(handlers.home).not.toHaveBeenCalled();
  });

  it.each(['#', '#/'])('hash %s falls back to #/home via normalizeHash', (hash) => {
    expect(normalizeHash(hash)).toBe('#/home');
  });

  it.each(['', '#', '#/', '#/unknown'])(
    'initRouter redirects %s to #/home and mounts home handler',
    (hash) => {
      hashValue = hash;
      initRouter(handlers);
      expect(hashValue).toBe('#/home');
      expect(handlers.home).toHaveBeenCalledTimes(1);
    },
  );

  it('back navigation from workbench returns to home hub', () => {
    initRouter(handlers);
    expect(handlers.home).toHaveBeenCalledTimes(1);

    hashValue = '#/workbench';
    listeners.hashchange();
    expect(handlers.workbench).toHaveBeenCalledTimes(1);

    handlers.home.mockClear();
    hashValue = '#/home';
    listeners.popstate();
    expect(handlers.home).toHaveBeenCalledTimes(1);
  });

  it('back navigation from corpus doc returns to home hub', () => {
    initRouter(handlers);
    expect(handlers.home).toHaveBeenCalledTimes(1);

    hashValue = '#/corpus/owner/repo';
    listeners.hashchange();
    expect(handlers['corpus-doc']).toHaveBeenCalledTimes(1);

    handlers.home.mockClear();
    hashValue = '#/home';
    listeners.popstate();
    expect(handlers.home).toHaveBeenCalledTimes(1);
  });
});

describe('read-later route navigation', () => {
  let handlers;
  let hashValue;
  let listeners;

  beforeEach(() => {
    handlers = {
      home: vi.fn(),
      'read-later': vi.fn(),
    };
    hashValue = '#/home';
    listeners = {};
    vi.stubGlobal('window', {
      addEventListener(type, fn) {
        listeners[type] = fn;
      },
      location: {
        get hash() {
          return hashValue;
        },
        set hash(value) {
          hashValue = value;
          listeners.hashchange?.();
        },
        replace(value) {
          const idx = value.indexOf('#');
          hashValue = idx >= 0 ? value.slice(idx) : value;
          listeners.hashchange?.();
        },
      },
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('initRouter invokes read-later handler for #/read-later', () => {
    hashValue = '#/read-later';
    initRouter(handlers, { fallback: '#/home' });
    expect(handlers['read-later']).toHaveBeenCalledTimes(1);
    expect(handlers['read-later']).toHaveBeenCalledWith({
      name: 'read-later',
      params: {},
    });
  });

  it('unknown hash still falls back to #/home without invoking read-later handler', () => {
    hashValue = '#/not-a-route';
    initRouter(handlers, { fallback: '#/home' });
    expect(hashValue).toBe('#/home');
    expect(handlers.home).toHaveBeenCalledTimes(1);
    expect(handlers['read-later']).not.toHaveBeenCalled();
  });
});

describe('navigateToNote / navigateBackToList (T3)', () => {
  let hashValue;
  let listeners;
  let historyBack;
  let historyStack;

  beforeEach(() => {
    hashValue = '#/workbench?date=20260719';
    listeners = {};
    historyStack = [hashValue];
    historyBack = vi.fn(() => {
      if (historyStack.length > 1) {
        historyStack.pop();
        hashValue = historyStack[historyStack.length - 1];
        listeners.popstate?.();
      }
    });
    vi.stubGlobal('window', {
      addEventListener(type, fn) {
        listeners[type] = fn;
      },
      history: {
        get length() {
          return historyStack.length;
        },
        back: historyBack,
      },
      location: {
        get hash() {
          return hashValue;
        },
        set hash(value) {
          hashValue = value;
          historyStack.push(value);
          listeners.hashchange?.();
        },
        replace(value) {
          const idx = value.indexOf('#');
          hashValue = idx >= 0 ? value.slice(idx) : value;
          historyStack[historyStack.length - 1] = hashValue;
          listeners.hashchange?.();
        },
      },
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('list→note writes #/workbench?date=&note= (+optional layer) via navigate history', () => {
    navigateToNote({
      date: '20260719',
      note: 'inbox/notes/x.md',
      layer: 'raw',
    });
    expect(hashValue).toBe(
      '#/workbench?date=20260719&note=inbox%2Fnotes%2Fx.md&layer=raw',
    );
    expect(historyStack.length).toBeGreaterThan(1);
    expect(parseHash(hashValue)).toEqual({
      name: 'workbench',
      params: {
        date: '20260719',
        note: 'inbox/notes/x.md',
        layer: 'raw',
      },
    });
  });

  it('list→note without layer omits layer query', () => {
    navigateToNote({ date: '20260719', note: 'inbox/notes/x.md' });
    expect(hashValue).toBe('#/workbench?date=20260719&note=inbox%2Fnotes%2Fx.md');
    expect(parseHash(hashValue).params).not.toHaveProperty('layer');
  });

  it('back prefers history.back between list↔note', () => {
    navigateToNote({ date: '20260719', note: 'inbox/notes/x.md' });
    expect(hashValue).toContain('note=');
    navigateBackToList({ date: '20260719' });
    expect(historyBack).toHaveBeenCalledTimes(1);
    expect(hashValue).toBe('#/workbench?date=20260719');
    expect(parseHash(hashValue).params).not.toHaveProperty('note');
  });

  it('without usable history, exit strips note and keeps date', () => {
    hashValue = '#/workbench?date=20260719&note=inbox%2Fnotes%2Fx.md';
    historyStack = [hashValue];
    navigateBackToList({ date: '20260719' });
    expect(historyBack).not.toHaveBeenCalled();
    expect(hashValue).toBe('#/workbench?date=20260719');
    expect(parseHash(hashValue)).toEqual({
      name: 'workbench',
      params: { date: '20260719' },
    });
  });

  it('create-in-progress back/close clears create local state and lands on list without temp note', () => {
    hashValue = '#/workbench?date=20260719';
    historyStack = [hashValue];
    const clearCreate = vi.fn();
    navigateBackToList({ date: '20260719', onClearCreate: clearCreate });
    expect(clearCreate).toHaveBeenCalledTimes(1);
    expect(hashValue).toBe('#/workbench?date=20260719');
    expect(parseHash(hashValue).params).not.toHaveProperty('note');
    expect(historyBack).not.toHaveBeenCalled();
  });

  it('navigate failure stays on list / safe empty — does not write half-open note location', () => {
    const before = hashValue;
    navigateToNote({ date: '20260719' }); // missing note
    expect(hashValue).toBe(before);
    expect(parseHash(hashValue).params).not.toHaveProperty('note');

    navigateToNote({ note: 'inbox/notes/x.md' }); // missing date
    expect(hashValue).toBe(before);
  });

  it('exit/back is allowed to mutate hash (abolishes exit-must-not-change-hash)', () => {
    hashValue = '#/workbench?date=20260719&note=inbox%2Fnotes%2Fx.md';
    historyStack = [hashValue];
    const before = hashValue;
    navigateBackToList({ date: '20260719' });
    expect(hashValue).not.toBe(before);
    expect(hashValue).toBe('#/workbench?date=20260719');
  });

  it('navigateToDateList strips note/layer and lands on chosen date (no history.back)', () => {
    navigateToNote({ date: '20260719', note: 'inbox/notes/x.md', layer: 'raw' });
    expect(hashValue).toContain('note=');
    navigateToDateList('20260718');
    expect(historyBack).not.toHaveBeenCalled();
    expect(hashValue).toBe('#/workbench?date=20260718');
    expect(parseHash(hashValue).params).not.toHaveProperty('note');
    expect(parseHash(hashValue).params).not.toHaveProperty('layer');
  });
});

