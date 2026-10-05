// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import {
  attachNotesSidebarResize,
  detachNotesSidebarResize,
  initSidebarResize,
  setSidebarWidth,
  STORAGE_KEY,
  MIN_WIDTH,
  MAX_WIDTH,
} from '../../frontend/src/notes/ui/sidebar-resize.ts';

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
    clear() {
      for (const key of Object.keys(store)) delete store[key];
    },
  };
}

describe('notes-sidebar-resize', () => {
  /** @type {HTMLElement} */
  let aside;

  beforeEach(() => {
    installLocalStorageMock();
    aside = document.createElement('aside');
    aside.id = 'sidebar';
    aside.style.width = '221px';
    const resizer = document.createElement('div');
    resizer.id = 'sidebar-resizer';
    resizer.className = 'sidebar-resizer';
    aside.appendChild(resizer);
    document.body.appendChild(aside);
  });

  afterEach(() => {
    detachNotesSidebarResize();
    aside.remove();
    document.body.classList.remove('sidebar-resizing');
  });

  it('restores width from localStorage', () => {
    localStorage.setItem(STORAGE_KEY, '300');
    attachNotesSidebarResize(aside);
    expect(aside.style.width).toBe('300px');
    expect(aside.style.getPropertyValue('--sidebar-width')).toBe('300px');
  });

  it('clamps width within min and max', () => {
    expect(setSidebarWidth(aside, MIN_WIDTH - 50)).toBe(MIN_WIDTH);
    expect(setSidebarWidth(aside, MAX_WIDTH + 100)).toBe(MAX_WIDTH);
  });

  it('persists width after drag ends', () => {
    aside.getBoundingClientRect = () => ({ width: 221 });
    attachNotesSidebarResize(aside);
    const resizer = aside.querySelector('#sidebar-resizer');

    resizer.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0, clientX: 100, pointerId: 1 }));
    document.dispatchEvent(new PointerEvent('pointermove', { clientX: 150, pointerId: 1 }));
    document.dispatchEvent(new PointerEvent('pointerup', { clientX: 150, pointerId: 1 }));

    expect(localStorage.getItem(STORAGE_KEY)).toBe(String(221 + 50));
    expect(document.body.classList.contains('sidebar-resizing')).toBe(false);
  });

  it('rebinds after remount so a new resizer still drags', () => {
    attachNotesSidebarResize(aside);
    aside.querySelector('#sidebar-resizer')?.remove();
    const next = document.createElement('div');
    next.id = 'sidebar-resizer';
    next.className = 'sidebar-resizer';
    aside.appendChild(next);
    aside.getBoundingClientRect = () => ({ width: 221 });
    attachNotesSidebarResize(aside);

    next.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0, clientX: 100, pointerId: 1 }));
    document.dispatchEvent(new PointerEvent('pointermove', { clientX: 140, pointerId: 1 }));
    document.dispatchEvent(new PointerEvent('pointerup', { clientX: 140, pointerId: 1 }));

    expect(localStorage.getItem(STORAGE_KEY)).toBe(String(221 + 40));
  });

  it('initSidebarResize rebinds after a first attach', () => {
    document.body.appendChild(aside);
    attachNotesSidebarResize(aside);
    aside.querySelector('#sidebar-resizer')?.remove();
    const next = document.createElement('div');
    next.id = 'sidebar-resizer';
    next.className = 'sidebar-resizer';
    aside.appendChild(next);
    aside.getBoundingClientRect = () => ({ width: 221 });
    initSidebarResize();

    next.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0, clientX: 80, pointerId: 1 }));
    document.dispatchEvent(new PointerEvent('pointermove', { clientX: 110, pointerId: 1 }));
    document.dispatchEvent(new PointerEvent('pointerup', { clientX: 110, pointerId: 1 }));

    expect(Number(localStorage.getItem(STORAGE_KEY))).toBeGreaterThan(221);
  });
});
