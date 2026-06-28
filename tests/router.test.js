import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { parseHash, initRouter, navigate } from '../frontend/js/router/index.js';

describe('parseHash', () => {
  it('parses #/workbench', () => {
    expect(parseHash('#/workbench')).toEqual({ name: 'workbench', params: {} });
  });

  it('parses #/home', () => {
    expect(parseHash('#/home')).toEqual({ name: 'home', params: {} });
  });

  it('parses #/corpus/pick', () => {
    expect(parseHash('#/corpus/pick')).toEqual({ name: 'corpus-pick', params: {} });
  });

  it('parses #/corpus/owner/repo as corpus-doc', () => {
    expect(parseHash('#/corpus/lulufoo/my-repo')).toEqual({
      name: 'corpus-doc',
      params: { repo: 'lulufoo/my-repo' },
    });
  });

  it('parses URL-encoded repo segment', () => {
    expect(parseHash('#/corpus/lulufoo%2Fmy-repo')).toEqual({
      name: 'corpus-doc',
      params: { repo: 'lulufoo/my-repo' },
    });
  });

  it('returns unknown for unrecognized paths', () => {
    expect(parseHash('#/unknown')).toEqual({ name: 'unknown', params: {} });
  });

  it('returns unknown for empty hash variants', () => {
    expect(parseHash('')).toEqual({ name: 'unknown', params: {} });
    expect(parseHash('#')).toEqual({ name: 'unknown', params: {} });
    expect(parseHash('#/')).toEqual({ name: 'unknown', params: {} });
  });
});

describe('initRouter', () => {
  let handlers;
  let hashValue;
  let listeners;

  beforeEach(() => {
    handlers = {
      workbench: vi.fn(),
      home: vi.fn(),
      'corpus-pick': vi.fn(),
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

  it('invokes workbench handler for #/workbench on init', () => {
    hashValue = '#/workbench';
    initRouter(handlers);
    expect(handlers.workbench).toHaveBeenCalledTimes(1);
  });

  it('redirects empty hash to fallback #/workbench', () => {
    hashValue = '';
    initRouter(handlers, { fallback: '#/workbench' });
    expect(hashValue).toBe('#/workbench');
    expect(handlers.workbench).toHaveBeenCalled();
  });

  it('redirects unknown hash to fallback #/workbench', () => {
    hashValue = '#/unknown';
    initRouter(handlers, { fallback: '#/workbench' });
    expect(hashValue).toBe('#/workbench');
    expect(handlers.workbench).toHaveBeenCalled();
  });

  it('re-invokes handler on hashchange', () => {
    hashValue = '#/workbench';
    initRouter(handlers);
    handlers.workbench.mockClear();
    hashValue = '#/home';
    listeners.hashchange();
    expect(handlers.home).toHaveBeenCalledTimes(1);
  });

  it('re-invokes handler on popstate', () => {
    hashValue = '#/workbench';
    initRouter(handlers);
    handlers.workbench.mockClear();
    hashValue = '#/corpus/pick';
    listeners.popstate();
    expect(handlers['corpus-pick']).toHaveBeenCalledTimes(1);
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
