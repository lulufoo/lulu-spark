// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import {
  attachKnowledgeSidebarResize,
  detachKnowledgeSidebarResize,
  setKnowledgeSidebarWidth,
  STORAGE_KEY,
  MIN_WIDTH,
  MAX_WIDTH,
} from '../../frontend/src/knowledge/ui/sidebar-resize.ts';

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

describe('knowledge-sidebar-resize', () => {
  /** @type {HTMLElement} */
  let aside;

  beforeEach(() => {
    installLocalStorageMock();
    aside = document.createElement('aside');
    aside.className = 'knowledge-doc-sidebar';
    aside.style.width = '360px';
    document.body.appendChild(aside);
  });

  afterEach(() => {
    detachKnowledgeSidebarResize();
    aside.remove();
    document.body.classList.remove('sidebar-resizing');
  });

  it('creates resizer separator on attach', () => {
    attachKnowledgeSidebarResize(aside);
    const resizer = aside.querySelector('.knowledge-sidebar-resizer.sidebar-resizer');
    expect(resizer).not.toBeNull();
    expect(resizer?.getAttribute('role')).toBe('separator');
  });

  it('restores width from localStorage', () => {
    localStorage.setItem(STORAGE_KEY, '420');
    attachKnowledgeSidebarResize(aside);
    expect(aside.style.width).toBe('420px');
    expect(aside.style.getPropertyValue('--knowledge-sidebar-width')).toBe('420px');
  });

  it('clamps width within min and max', () => {
    expect(setKnowledgeSidebarWidth(aside, MIN_WIDTH - 50)).toBe(MIN_WIDTH);
    expect(setKnowledgeSidebarWidth(aside, MAX_WIDTH + 100)).toBe(MAX_WIDTH);
  });

  it('persists width after drag ends', () => {
    aside.getBoundingClientRect = () => ({ width: 360 });
    attachKnowledgeSidebarResize(aside);
    const resizer = aside.querySelector('.knowledge-sidebar-resizer');
    expect(resizer).not.toBeNull();

    const pointerOpts = { bubbles: true, button: 0, clientX: 100, pointerId: 1 };
    resizer.dispatchEvent(new PointerEvent('pointerdown', pointerOpts));
    document.dispatchEvent(new PointerEvent('pointermove', { clientX: 150, pointerId: 1 }));
    document.dispatchEvent(new PointerEvent('pointerup', { clientX: 150, pointerId: 1 }));

    expect(localStorage.getItem(STORAGE_KEY)).toBe(String(360 + 50));
    expect(document.body.classList.contains('sidebar-resizing')).toBe(false);
  });

  it('detach removes listeners without leaving resizing state', () => {
    const cleanup = attachKnowledgeSidebarResize(aside);
    cleanup();
    const resizer = aside.querySelector('.knowledge-sidebar-resizer');
    resizer?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0, clientX: 0 }));
    expect(document.body.classList.contains('sidebar-resizing')).toBe(false);
  });
});
