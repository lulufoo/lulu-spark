// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import {
  restoreSidebarCollapsed,
  toggleSidebarCollapsed,
} from '../../frontend/src/app-shell/commands/sidebar-collapsed.ts';
import {
  SIDEBAR_COLLAPSED_KEY,
  sidebarCollapsedStore,
} from '../../frontend/src/app-shell/state/sidebar-collapsed.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const NOTES_WIDTH_KEY = 'cta_sidebar_width';
const HOME_WIDTH_KEY = 'home_chat_sidebar_width';

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

describe('sidebar collapsed commands', () => {
  beforeEach(() => {
    originalStorage = globalThis.localStorage;
    installStorage(memoryStorage({
      spark_workspace: 'android',
      [NOTES_WIDTH_KEY]: '221',
      [HOME_WIDTH_KEY]: '248',
    }));
    sidebarCollapsedStore.set(false);
  });

  afterEach(() => {
    installStorage(originalStorage);
    sidebarCollapsedStore.set(false);
  });

  it('toggles back and forth and saves each value', () => {
    expect(toggleSidebarCollapsed()).toBe(true);
    expect(sidebarCollapsedStore.getSnapshot()).toBe(true);
    expect(globalThis.localStorage.getItem(SIDEBAR_COLLAPSED_KEY)).toBe('true');

    expect(toggleSidebarCollapsed()).toBe(false);
    expect(sidebarCollapsedStore.getSnapshot()).toBe(false);
    expect(globalThis.localStorage.getItem(SIDEBAR_COLLAPSED_KEY)).toBe('false');

    expect(toggleSidebarCollapsed()).toBe(true);
  });

  it('restores the saved flag into a fresh session', () => {
    globalThis.localStorage.setItem(SIDEBAR_COLLAPSED_KEY, 'true');
    sidebarCollapsedStore.set(false);

    expect(restoreSidebarCollapsed()).toBe(true);
    expect(sidebarCollapsedStore.getSnapshot()).toBe(true);
  });

  it('still toggles in the session when a write throws', () => {
    installStorage({
      getItem() {
        return null;
      },
      setItem() {
        throw new Error('quota');
      },
    });

    expect(() => toggleSidebarCollapsed()).not.toThrow();
    expect(sidebarCollapsedStore.getSnapshot()).toBe(true);
    expect(() => toggleSidebarCollapsed()).not.toThrow();
    expect(sidebarCollapsedStore.getSnapshot()).toBe(false);
  });

  it('leaves the saved workspace id and both width prefs alone', () => {
    toggleSidebarCollapsed();
    restoreSidebarCollapsed();

    expect(globalThis.localStorage.getItem('spark_workspace')).toBe('android');
    expect(globalThis.localStorage.getItem(NOTES_WIDTH_KEY)).toBe('221');
    expect(globalThis.localStorage.getItem(HOME_WIDTH_KEY)).toBe('248');
  });

  it('does not import navigation or selection modules', () => {
    const source = readFileSync(
      join(repoRoot, 'frontend/src/app-shell/commands/sidebar-collapsed.ts'),
      'utf8',
    );
    expect(source).not.toMatch(/routes|workspace|project|host\/api/i);
  });
});
