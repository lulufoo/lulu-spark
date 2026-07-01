import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import {
  parseHash,
  initRouter,
  navigate,
  normalizeHash,
} from '../frontend/js/router/index.js';

describe('parseHash', () => {
  it('parses #/workbench', () => {
    expect(parseHash('#/workbench')).toEqual({ name: 'workbench', params: {} });
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
