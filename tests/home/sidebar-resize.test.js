// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import {
  attachHomeSidebarResize,
  detachHomeSidebarResize,
  setHomeSidebarWidth,
  STORAGE_KEY,
  MIN_WIDTH,
  MAX_WIDTH,
} from '../../frontend/src/home/ui/sidebar-resize.ts';

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

describe('home-sidebar-resize', () => {
  /** @type {HTMLElement} */
  let aside;

  beforeEach(() => {
    installLocalStorageMock();
    aside = document.createElement('aside');
    aside.className = 'home-chat-sidebar';
    aside.style.width = '248px';
    document.body.appendChild(aside);
  });

  afterEach(() => {
    detachHomeSidebarResize();
    aside.remove();
    document.body.classList.remove('sidebar-resizing');
  });

  it('creates resizer separator on attach', () => {
    attachHomeSidebarResize(aside);
    const resizer = aside.querySelector('.home-chat-sidebar-resizer.sidebar-resizer');
    expect(resizer).not.toBeNull();
    expect(resizer?.getAttribute('role')).toBe('separator');
  });

  it('restores width from localStorage', () => {
    localStorage.setItem(STORAGE_KEY, '320');
    attachHomeSidebarResize(aside);
    expect(aside.style.width).toBe('320px');
    expect(aside.style.getPropertyValue('--home-chat-sidebar-width')).toBe('320px');
  });

  it('clamps width within min and max', () => {
    expect(setHomeSidebarWidth(aside, MIN_WIDTH - 50)).toBe(MIN_WIDTH);
    expect(setHomeSidebarWidth(aside, MAX_WIDTH + 100)).toBe(MAX_WIDTH);
  });

  it('persists width after drag ends', () => {
    aside.getBoundingClientRect = () => ({ width: 248 });
    attachHomeSidebarResize(aside);
    const resizer = aside.querySelector('.home-chat-sidebar-resizer');
    expect(resizer).not.toBeNull();

    const pointerOpts = { bubbles: true, button: 0, clientX: 100, pointerId: 1 };
    resizer.dispatchEvent(new PointerEvent('pointerdown', pointerOpts));
    document.dispatchEvent(new PointerEvent('pointermove', { clientX: 150, pointerId: 1 }));
    document.dispatchEvent(new PointerEvent('pointerup', { clientX: 150, pointerId: 1 }));

    expect(localStorage.getItem(STORAGE_KEY)).toBe(String(248 + 50));
    expect(document.body.classList.contains('sidebar-resizing')).toBe(false);
  });

  it('detach removes listeners without leaving resizing state', () => {
    const cleanup = attachHomeSidebarResize(aside);
    cleanup();
    const resizer = aside.querySelector('.home-chat-sidebar-resizer');
    resizer?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0, clientX: 0 }));
    expect(document.body.classList.contains('sidebar-resizing')).toBe(false);
  });
});
