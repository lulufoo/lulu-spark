// @vitest-environment jsdom
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';
import * as hub from '../../frontend/src/home/commands/hub.ts';
import { mountHomeHub } from '../../frontend/src/home/hub.tsx';
import * as store from '../../frontend/src/home/state/store.ts';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const css = readFileSync(join(root, 'frontend/app.css'), 'utf8');

function sessionsOf(count) {
  return Array.from({ length: count }, (_, i) => ({
    session_id: `s${i + 1}`,
    title: `Chat ${i + 1}`,
    updated_at: 1_760_000_000 - i * 60,
  }));
}

describe('chat list is not paged', () => {
  it('the paging state, constants and reveal helpers are gone', () => {
    expect(store.HOME_CHAT_FIRST_PAGE).toBeUndefined();
    expect(store.HOME_CHAT_PAGE_STEP).toBeUndefined();
    expect(store.visibleSessionCount).toBeUndefined();
    expect(hub.showMoreSessions).toBeUndefined();
    expect('sessionsShown' in store.getHomeState()).toBe(false);
    expect(existsSync(join(root, 'frontend/src/home/ui/reveal-more-chats.ts'))).toBe(false);
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
    container.remove();
    delete window.__TAURI__;
  });

  const list = () => container.querySelector('[data-role="session-list"]');
  const rows = () => list().querySelectorAll('.home-chat-session').length;

  const mount = () => {
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
  };

  it('renders every chat the host returns at once, and scrolling reveals nothing more', async () => {
    mount();
    await vi.waitFor(() => expect(rows()).toBe(45));
    list().dispatchEvent(new Event('scroll', { bubbles: true }));
    await new Promise((resolve) => setTimeout(resolve, 30));
    expect(rows()).toBe(45);
  });

  it('shows all of them when they are few', async () => {
    total = 4;
    mount();
    await vi.waitFor(() => expect(rows()).toBe(4));
  });

  it('lists the current chat like any other', async () => {
    currentId = 's27';
    mount();
    await vi.waitFor(() => expect(rows()).toBe(45));
    expect(list().querySelector('[data-session-id="s27"]')).not.toBeNull();
  });

  it('picking a chat in the list selects it', async () => {
    mount();
    await vi.waitFor(() => expect(rows()).toBe(45));
    list().querySelector('[data-session-id="s8"]').click();
    await vi.waitFor(() => expect(invokeSpy).toHaveBeenCalledWith('select_chat_session', { sessionId: 's8' }));
  });

  it('has no history icon and no All chats popover; the header is just the title', async () => {
    mount();
    await vi.waitFor(() => expect(rows()).toBe(45));
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
