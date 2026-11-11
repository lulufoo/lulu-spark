// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import {
  SIDEBAR_COLLAPSED_KEY,
  recallSidebarCollapsed,
  rememberSidebarCollapsed,
  sidebarCollapsedStore,
} from '../../frontend/src/app-shell/state/sidebar-collapsed.ts';

/** @type {Storage} */
let originalStorage;

function installStorage(storage) {
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: storage,
  });
}

function memoryStorage(seed = {}) {
  const data = { ...seed };
  return {
    getItem(key) {
      return Object.prototype.hasOwnProperty.call(data, key) ? data[key] : null;
    },
    setItem(key, value) {
      data[key] = String(value);
    },
    removeItem(key) {
      delete data[key];
    },
  };
}

describe('sidebar collapsed preference', () => {
  beforeEach(() => {
    originalStorage = globalThis.localStorage;
    installStorage(memoryStorage());
    sidebarCollapsedStore.set(false);
  });

  afterEach(() => {
    installStorage(originalStorage);
    sidebarCollapsedStore.set(false);
  });

  it('starts expanded when no preference is stored', () => {
    expect(sidebarCollapsedStore.getSnapshot()).toBe(false);
    expect(recallSidebarCollapsed()).toBe(false);
  });

  it('restores an explicit true or false and treats any other stored value as expanded', () => {
    installStorage(memoryStorage({ [SIDEBAR_COLLAPSED_KEY]: 'true' }));
    expect(recallSidebarCollapsed()).toBe(true);

    installStorage(memoryStorage({ [SIDEBAR_COLLAPSED_KEY]: 'false' }));
    expect(recallSidebarCollapsed()).toBe(false);

    for (const raw of ['', '1', 'TRUE', 'yes']) {
      installStorage(memoryStorage({ [SIDEBAR_COLLAPSED_KEY]: raw }));
      expect(recallSidebarCollapsed()).toBe(false);
    }
  });

  it('remembers the session value and writes the same flag', () => {
    rememberSidebarCollapsed(true);
    expect(sidebarCollapsedStore.getSnapshot()).toBe(true);
    expect(globalThis.localStorage.getItem(SIDEBAR_COLLAPSED_KEY)).toBe('true');

    rememberSidebarCollapsed(false);
    expect(sidebarCollapsedStore.getSnapshot()).toBe(false);
    expect(globalThis.localStorage.getItem(SIDEBAR_COLLAPSED_KEY)).toBe('false');
  });

  it('keeps the session value when storage cannot be read or written', () => {
    sidebarCollapsedStore.set(true);
    installStorage({
      getItem() {
        throw new Error('storage blocked');
      },
      setItem() {
        throw new Error('storage blocked');
      },
    });

    expect(recallSidebarCollapsed()).toBe(true);
    expect(() => rememberSidebarCollapsed(false)).not.toThrow();
    expect(sidebarCollapsedStore.getSnapshot()).toBe(false);

    installStorage(undefined);
    sidebarCollapsedStore.set(true);
    expect(recallSidebarCollapsed()).toBe(true);
    expect(() => rememberSidebarCollapsed(false)).not.toThrow();
    expect(sidebarCollapsedStore.getSnapshot()).toBe(false);
  });
});
