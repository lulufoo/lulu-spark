// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
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

  beforeEach(() => {
    window.__TAURI__ = {};
    container = document.createElement('div');
    document.body.appendChild(container);
    channelSpy = vi.spyOn(api, 'createChannel').mockImplementation(async (onmessage) => ({ onmessage }));
    invokeSpy = vi.spyOn(api, 'invoke').mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') return { sessions: sessionsOf(45), current_session_id: '' };
      if (cmd === 'get_ai_assistant_binding') return { session_id: '', turns: [] };
      return {};
    });
  });

  afterEach(() => {
    cleanup?.();
    channelSpy.mockRestore();
    invokeSpy.mockRestore();
    container.remove();
    delete window.__TAURI__;
  });

  const rows = () => container.querySelectorAll('.home-chat-session').length;

  it('lists 10, then 10 more each time the list is scrolled to the bottom, past the old 20 limit', async () => {
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
    await vi.waitFor(() => expect(rows()).toBe(10));
    const list = container.querySelector('[data-role="session-list"]');
    const metrics = { scrollHeight: 400, clientHeight: 200, scrollTop: 0 };
    for (const key of Object.keys(metrics)) {
      Object.defineProperty(list, key, { configurable: true, get: () => metrics[key] });
    }

    list.dispatchEvent(new Event('scroll', { bubbles: true }));
    expect(rows()).toBe(10);

    metrics.scrollTop = 200;
    list.dispatchEvent(new Event('scroll', { bubbles: true }));
    await vi.waitFor(() => expect(rows()).toBe(20));
    list.dispatchEvent(new Event('scroll', { bubbles: true }));
    await vi.waitFor(() => expect(rows()).toBe(30));
    list.dispatchEvent(new Event('scroll', { bubbles: true }));
    list.dispatchEvent(new Event('scroll', { bubbles: true }));
    await vi.waitFor(() => expect(rows()).toBe(45));
    list.dispatchEvent(new Event('scroll', { bubbles: true }));
    expect(rows()).toBe(45);
  });

  it('keeps the empty-state text when there are no chats', async () => {
    invokeSpy.mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') return { sessions: [], current_session_id: '' };
      return {};
    });
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
    await vi.waitFor(() => {
      expect(container.querySelector('.home-chat-sessions-empty')?.textContent).toBe('No conversations yet.');
    });
  });
});
