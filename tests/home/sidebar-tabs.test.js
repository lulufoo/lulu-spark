// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';
import { mountHomeHub } from '../../frontend/src/home/hub.tsx';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

describe('Home sidebar tabs', () => {
  let container;
  let cleanup;
  let invokeSpy;
  let createChannelSpy;
  let sessions;
  let currentId;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    cleanup = null;
    window.__TAURI__ = { event: { listen: vi.fn(async () => vi.fn()) } };
    sessions = [
      { session_id: 's1', title: 'Hello from first' },
      { session_id: 's2', title: 'Second chat' },
    ];
    currentId = 's2';
    createChannelSpy = vi.spyOn(api, 'createChannel').mockImplementation(async (onmessage) => ({ onmessage }));
    invokeSpy = vi.spyOn(api, 'invoke').mockImplementation(async (cmd, args) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') return { sessions, current_session_id: currentId };
      if (cmd === 'select_chat_session') {
        currentId = String(args?.sessionId || '');
        return { session_id: currentId, turns: [] };
      }
      if (cmd === 'create_chat_session') {
        currentId = 's3';
        sessions = [{ session_id: 's3', title: 'New conversation' }, ...sessions];
        return { session_id: 's3', turns: [] };
      }
      return {};
    });
  });

  afterEach(() => {
    cleanup?.();
    createChannelSpy.mockRestore();
    invokeSpy.mockRestore();
    container.remove();
    delete window.__TAURI__;
  });

  async function mountReady() {
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
    await vi.waitFor(() => {
      expect(container.querySelectorAll('.home-chat-session').length).toBe(2);
    });
  }

  const tab = (pane) => container.querySelector(`.home-sidebar-tab[data-pane="${pane}"]`);
  const nav = () => container.querySelector('.home-chat-nav');
  const pane = () => container.querySelector('.home-chat-pane');

  it('opens on Home: workspace list shown, chats hidden', async () => {
    await mountReady();
    expect(tab('home').getAttribute('aria-selected')).toBe('true');
    expect(tab('chat').getAttribute('aria-selected')).toBe('false');
    expect(nav().hidden).toBe(false);
    expect(pane().hidden).toBe(true);
  });

  it('names the tabs Home and Spark AI, with the search button after them', async () => {
    await mountReady();
    const row = container.querySelector('.home-sidebar-tabs');
    expect([...row.querySelectorAll('.home-sidebar-tab-label')].map((el) => el.textContent)).toEqual([
      'Home',
      'Spark AI',
    ]);
    expect(row.lastElementChild.getAttribute('data-role')).toBe('sidebar-search');
  });

  it('switches panes when a tab is clicked, and back', async () => {
    await mountReady();
    tab('chat').click();
    await vi.waitFor(() => {
      expect(pane().hidden).toBe(false);
    });
    expect(nav().hidden).toBe(true);
    expect(tab('chat').getAttribute('aria-selected')).toBe('true');
    tab('home').click();
    await vi.waitFor(() => {
      expect(nav().hidden).toBe(false);
    });
    expect(pane().hidden).toBe(true);
  });

  it('keeps both panes mounted so the workspace list can measure itself', async () => {
    await mountReady();
    expect(nav().querySelector('.home-nav-workspaces')).not.toBeNull();
    expect(pane().querySelector('[data-role="session-list"]')).not.toBeNull();
  });

  it('has no Chats header, so no "+" or search sits above the list', async () => {
    await mountReady();
    expect(container.querySelector('.home-chat-sessions-head')).toBeNull();
    expect(pane().querySelector('[data-role="new-session"]')).toBeNull();
    expect(pane().querySelector('.home-chat-new')).toBeNull();
  });

  it('pins New chat above the account row, outside both panes', async () => {
    await mountReady();
    const dock = container.querySelector('[data-role="new-session"]');
    expect(dock.textContent).toBe('New chat⌘O');
    expect(dock.closest('.home-chat-nav')).toBeNull();
    expect(dock.closest('.home-chat-pane')).toBeNull();
    expect(dock.nextElementSibling.getAttribute('data-role')).toBe('account-row');
  });

  it('New chat creates a session and switches to the Spark AI tab', async () => {
    await mountReady();
    expect(pane().hidden).toBe(true);
    container.querySelector('[data-role="new-session"]').click();
    await vi.waitFor(() => {
      expect(invokeSpy).toHaveBeenCalledWith('create_chat_session');
      expect(container.querySelector('[data-session-id="s3"]')).not.toBeNull();
    });
    await vi.waitFor(() => {
      expect(pane().hidden).toBe(false);
    });
    expect(tab('chat').getAttribute('aria-selected')).toBe('true');
    expect(nav().hidden).toBe(true);
  });

  it('keeps the sidebar resize handle', async () => {
    await mountReady();
    expect(container.querySelector('.home-chat-sidebar > .home-chat-sidebar-resizer.sidebar-resizer')).not.toBeNull();
  });
});

describe('Home sidebar tabs (styles)', () => {
  const css = readFileSync(join(repoRoot, 'frontend/home.css'), 'utf8');

  it('collapses the unselected tab to an icon and expands the selected one', () => {
    expect(css).toMatch(/\.home-sidebar-tab-label \{[^}]*max-width: 0;/);
    expect(css).toMatch(/\.home-sidebar-tab\.is-active \.home-sidebar-tab-label \{[^}]*max-width: 5\.9em;/);
  });

  it('lets the chat list fill the pane instead of a fixed row count', () => {
    expect(css).toMatch(/\.home-chat-pane \.home-chat-sessions \{[^}]*flex: 1 1 0;[^}]*height: auto;/);
  });

  it('hides an inactive pane even though the pane rules set display', () => {
    expect(css).toMatch(/\.home-chat-nav\[hidden\],\s*\.home-chat-pane\[hidden\] \{ display: none; \}/);
  });
});
