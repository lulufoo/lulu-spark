// @vitest-environment jsdom
import { act } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';
import { mountHomeHub } from '../../frontend/src/home/hub.tsx';
import { sidebarCollapsedStore } from '../../frontend/src/app-shell/state/sidebar-collapsed.ts';
import { homeSidebarWidthStore } from '../../frontend/src/home/state/sidebar-width.ts';

const WIDTH_KEY = 'home_chat_sidebar_width';

describe('home sidebar collapse', () => {
  let container;
  let cleanup;
  let originalStorage;

  beforeEach(() => {
    globalThis.IS_REACT_ACT_ENVIRONMENT = true;
    originalStorage = globalThis.localStorage;
    const store = { [WIDTH_KEY]: '180' };
    globalThis.localStorage = {
      getItem: (key) => (Object.prototype.hasOwnProperty.call(store, key) ? store[key] : null),
      setItem: (key, value) => {
        store[key] = String(value);
      },
      removeItem: (key) => {
        delete store[key];
      },
    };
    sidebarCollapsedStore.set(false);
    homeSidebarWidthStore.set(248);
    container = document.createElement('div');
    document.body.appendChild(container);
    cleanup = null;
    window.__TAURI__ = { event: { listen: vi.fn(async () => vi.fn()) } };
    const sessions = [
      { session_id: 's1', title: 'Hello from first' },
      { session_id: 's2', title: 'Second chat' },
    ];
    let currentId = 's2';
    vi.spyOn(api, 'createChannel').mockImplementation(async (onmessage) => ({ onmessage }));
    vi.spyOn(api, 'invoke').mockImplementation(async (cmd, args) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') return { sessions, current_session_id: currentId };
      if (cmd === 'select_chat_session') {
        currentId = String(args?.sessionId || '');
        return { session_id: currentId, turns: [] };
      }
      return {};
    });
  });

  afterEach(() => {
    cleanup?.();
    vi.restoreAllMocks();
    container.remove();
    delete window.__TAURI__;
    sidebarCollapsedStore.set(false);
    homeSidebarWidthStore.set(248);
    globalThis.localStorage = originalStorage;
  });

  async function mountReady(chrome = {}) {
    cleanup = mountHomeHub(container, { navigate: vi.fn(), openReadLater: vi.fn(), ...chrome });
    await vi.waitFor(() => {
      expect(container.querySelectorAll('.home-chat-session').length).toBe(2);
    });
  }

  it('hides the nav without dropping the chat draft, tab, or saved width', async () => {
    const openReadLater = vi.fn();
    await mountReady({ openReadLater });

    container.querySelector('.home-sidebar-tab[data-pane="chat"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.home-chat-pane').hidden).toBe(false);
    });
    const input = container.querySelector('[data-role="input"]');
    const nav = container.querySelector('.home-chat-nav');
    const sessions = container.querySelector('[data-role="session-list"]');
    input.textContent = 'keep this draft';
    nav.scrollTop = 36;

    act(() => {
      container.querySelector('[data-sidebar-toggle]').click();
    });

    const lower = container.querySelector('#home-sidebar-nav');
    const readLater = container.querySelector('[data-home-entry="read-later"]');
    expect(lower.hidden).toBe(true);
    expect(lower.contains(readLater)).toBe(false);
    expect(readLater.closest('[hidden]')).toBeNull();
    expect(container.querySelector('[aria-label="Settings"]').closest('[hidden]')).toBeNull();
    expect(container.querySelector('[data-role="input"]')).toBe(input);
    expect(input.textContent).toBe('keep this draft');
    expect(container.querySelector('[data-role="session-list"]')).toBe(sessions);
    expect(container.querySelector('.home-chat-nav')).toBe(nav);
    expect(nav.scrollTop).toBe(36);
    expect(container.querySelector('.home-sidebar-tab[data-pane="chat"]').getAttribute('aria-selected')).toBe('true');
    expect(localStorage.getItem(WIDTH_KEY)).toBe('180');
    expect(localStorage.getItem('sidebar_collapsed')).toBe('true');
    expect(container.querySelector('.sidebar-layout').style.getPropertyValue('--sidebar-layout-width')).toBe('180px');
    expect(container.querySelector('.home-chat-sidebar-resizer').hidden).toBe(true);

    readLater.click();
    expect(openReadLater).toHaveBeenCalledOnce();

    act(() => {
      container.querySelector('[data-sidebar-toggle]').click();
    });
    expect(container.querySelector('#home-sidebar-nav').hidden).toBe(false);
    expect(container.querySelector('[data-role="input"]')).toBe(input);
    expect(input.textContent).toBe('keep this draft');
  });
});
