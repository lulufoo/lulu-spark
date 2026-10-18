// @vitest-environment jsdom
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';
import { showMoreSessions } from '../../frontend/src/home/commands/hub.ts';
import { mountHomeHub } from '../../frontend/src/home/hub.tsx';
import {
  getHomeState,
  HOME_CHAT_FIRST_PAGE,
  HOME_CHAT_PAGE_STEP,
  resetHomeState,
  setHomeState,
  visibleSessionCount,
} from '../../frontend/src/home/state/store.ts';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const css = readFileSync(join(root, 'frontend/app.css'), 'utf8');

function sessionsOf(count) {
  return Array.from({ length: count }, (_, i) => ({ session_id: `s${i + 1}`, title: `Chat ${i + 1}` }));
}

function seed(count, patch = {}) {
  setHomeState((prev) => ({
    ...prev,
    sessions: sessionsOf(count),
    sessionsShown: HOME_CHAT_FIRST_PAGE,
    currentSessionId: '',
    ...patch,
  }));
}

describe('chat paging state', () => {
  beforeEach(() => resetHomeState());
  afterEach(() => resetHomeState());

  it('first shows 10 and reveals 10 more each time, never past the total', () => {
    expect(HOME_CHAT_FIRST_PAGE).toBe(10);
    expect(HOME_CHAT_PAGE_STEP).toBe(10);
    seed(25);
    expect(visibleSessionCount(getHomeState())).toBe(10);
    showMoreSessions();
    expect(visibleSessionCount(getHomeState())).toBe(20);
    showMoreSessions();
    expect(visibleSessionCount(getHomeState())).toBe(25);
    showMoreSessions();
    expect(visibleSessionCount(getHomeState())).toBe(25);
    expect(getHomeState().sessionsShown).toBe(25);
  });

  it('shows everything when there are fewer than a page', () => {
    seed(3);
    expect(visibleSessionCount(getHomeState())).toBe(3);
    showMoreSessions();
    expect(getHomeState().sessionsShown).toBe(HOME_CHAT_FIRST_PAGE);
  });

  it('stretches to include a current chat that sits beyond the revealed range', () => {
    seed(40, { currentSessionId: 's27' });
    expect(visibleSessionCount(getHomeState())).toBe(27);
    showMoreSessions();
    expect(visibleSessionCount(getHomeState())).toBe(37);
  });

  it('stays within bounds when chats are deleted', () => {
    seed(25, { sessionsShown: 30 });
    expect(visibleSessionCount(getHomeState())).toBe(25);
    setHomeState((prev) => ({ ...prev, sessions: prev.sessions.slice(0, 12) }));
    expect(visibleSessionCount(getHomeState())).toBe(12);
  });
});

describe('Chats block layout', () => {
  it('is a fixed five rows tall and no longer grows with the window', () => {
    const block = css.match(/\.home-chat-sessions \{[^}]*\}/)?.[0] ?? '';
    expect(block).toMatch(/flex: 0 0 auto/);
    expect(block).toMatch(/height: calc\(var\(--home-chat-list-rows\) \* var\(--home-chat-row-h\)/);
    expect(block).toMatch(/overflow-y: auto/);
    expect(block).not.toMatch(/flex: 1/);
    expect(css).toMatch(/--home-chat-list-rows: 5;/);
  });

  it('gives every chat row the same fixed height', () => {
    const row = css.match(/\.home-chat-session \{[^}]*\}/)?.[0] ?? '';
    expect(row).toMatch(/height: var\(--home-chat-row-h\)/);
  });
});

describe('Chats list in the sidebar', () => {
  let container;
  let cleanup;
  let invokeSpy;
  let channelSpy;
  let total;
  let currentId;

  // jsdom has no layout: give the chat list a height, and a content height of 36px per row, so the
  // list can be "full" or "short". A list inside a hidden pane has no height (display:none).
  const LIST_HEIGHT = 500;
  const ROW_PX = 36;
  const realClient = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'clientHeight');
  const realScroll = Object.getOwnPropertyDescriptor(Element.prototype, 'scrollHeight');

  function stubLayout() {
    Object.defineProperty(HTMLElement.prototype, 'clientHeight', {
      configurable: true,
      get() {
        if (!this.matches('[data-role="session-list"]')) return 0;
        return this.closest('[hidden]') ? 0 : LIST_HEIGHT;
      },
    });
    Object.defineProperty(HTMLElement.prototype, 'scrollHeight', {
      configurable: true,
      get() {
        return this.matches('[data-role="session-list"]')
          ? this.querySelectorAll('.home-chat-session').length * ROW_PX
          : 0;
      },
    });
  }

  beforeEach(() => {
    window.__TAURI__ = {};
    total = 45;
    currentId = '';
    container = document.createElement('div');
    document.body.appendChild(container);
    channelSpy = vi.spyOn(api, 'createChannel').mockImplementation(async (onmessage) => ({ onmessage }));
    invokeSpy = vi.spyOn(api, 'invoke').mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') return { sessions: sessionsOf(total), current_session_id: currentId };
      if (cmd === 'get_ai_assistant_binding') return { session_id: currentId, turns: [] };
      return {};
    });
  });

  afterEach(() => {
    cleanup?.();
    channelSpy.mockRestore();
    invokeSpy.mockRestore();
    if (realClient) Object.defineProperty(HTMLElement.prototype, 'clientHeight', realClient);
    else delete HTMLElement.prototype.clientHeight;
    if (realScroll) Object.defineProperty(Element.prototype, 'scrollHeight', realScroll);
    delete HTMLElement.prototype.scrollHeight;
    container.remove();
    delete window.__TAURI__;
  });

  const list = () => container.querySelector('[data-role="session-list"]');
  const rows = () => list().querySelectorAll('.home-chat-session').length;

  function setScroll(el, metrics) {
    for (const key of Object.keys(metrics)) {
      Object.defineProperty(el, key, { configurable: true, get: () => metrics[key] });
    }
  }

  const mount = () => {
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
  };
  const showChatsTab = () => container.querySelector('.home-sidebar-tab[data-pane="chat"]').click();

  it('lists every chat, a page at a time: first 10 of 45, 10 more each time it is scrolled to the bottom', async () => {
    mount();
    await vi.waitFor(() => expect(rows()).toBe(10));

    // The content grows as chats are revealed, like a real list; the scroll position stays put.
    const el = list();
    let scrollTop = 0;
    Object.defineProperty(el, 'clientHeight', { configurable: true, get: () => 200 });
    Object.defineProperty(el, 'scrollHeight', {
      configurable: true,
      get: () => el.querySelectorAll('.home-chat-session').length * ROW_PX,
    });
    Object.defineProperty(el, 'scrollTop', { configurable: true, get: () => scrollTop });
    const scrollToBottom = () => {
      scrollTop = el.scrollHeight - 200;
      el.dispatchEvent(new Event('scroll', { bubbles: true }));
    };

    scrollTop = 20;
    el.dispatchEvent(new Event('scroll', { bubbles: true }));
    expect(rows()).toBe(10);

    scrollToBottom();
    await vi.waitFor(() => expect(rows()).toBe(20));
    scrollToBottom();
    await vi.waitFor(() => expect(rows()).toBe(30));
    scrollToBottom();
    await vi.waitFor(() => expect(rows()).toBe(40));
    scrollToBottom();
    await vi.waitFor(() => expect(rows()).toBe(45));
    scrollToBottom();
    expect(rows()).toBe(45);
  });

  it('reveals the next page by itself while the revealed chats do not fill the list', async () => {
    stubLayout();
    mount();
    showChatsTab();
    // 500px of list: 10 rows = 360px is short, so it grows to 20 rows = 720px, which fills it.
    await vi.waitFor(() => expect(rows()).toBe(20));
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(rows()).toBe(20);
  });

  it('shows all of them, with no scrolling needed, when they are few', async () => {
    stubLayout();
    total = 4;
    mount();
    showChatsTab();
    await vi.waitFor(() => expect(rows()).toBe(4));
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(rows()).toBe(4);
  });

  it('reveals nothing while the Spark AI pane is hidden, and starts once it is shown', async () => {
    stubLayout();
    mount();
    await vi.waitFor(() => expect(rows()).toBe(10));
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(rows()).toBe(10);
    showChatsTab();
    await vi.waitFor(() => expect(rows()).toBe(20));
  });

  it('keeps the current chat in the list even when it is beyond the first page', async () => {
    currentId = 's27';
    mount();
    await vi.waitFor(() => expect(rows()).toBe(27));
    expect(list().querySelector('[data-session-id="s27"]')).not.toBeNull();
  });

  it('picking a chat in the list selects it', async () => {
    mount();
    await vi.waitFor(() => expect(rows()).toBe(10));
    list().querySelector('[data-session-id="s8"]').click();
    await vi.waitFor(() => expect(invokeSpy).toHaveBeenCalledWith('select_chat_session', { sessionId: 's8' }));
  });

  it('has no history icon and no All chats popover; the header is just the title', async () => {
    mount();
    await vi.waitFor(() => expect(rows()).toBe(10));
    const head = container.querySelector('.home-chat-pane .home-chat-sessions-head');
    expect([...head.children].map((el) => el.className)).toEqual(['home-chat-sessions-title']);
    expect(container.querySelector('[data-role="history-trigger"]')).toBeNull();
    expect(container.querySelector('[data-role="history-panel"]')).toBeNull();
    expect(container.querySelector('.home-chat-sessions-actions')).toBeNull();
  });

  it('the popover code and styles are deleted', () => {
    expect(existsSync(join(root, 'frontend/src/home/ui/session-history.tsx'))).toBe(false);
    expect(existsSync(join(root, 'frontend/src/home/state/sidebar-sessions.ts'))).toBe(false);
    const home = readFileSync(join(root, 'frontend/home.css'), 'utf8');
    expect(home).not.toMatch(/\.home-history|\.home-chat-sessions-actions/);
  });

  it('keeps the empty-state text when there are no chats', async () => {
    total = 0;
    mount();
    await vi.waitFor(() => {
      expect(container.querySelector('.home-chat-sessions-empty')?.textContent).toBe('No conversations yet.');
    });
  });
});
