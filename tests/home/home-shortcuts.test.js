// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';
import { mountHomeHub } from '../../frontend/src/home/hub.tsx';
import { attachHomeShortcuts, detachHomeShortcuts } from '../../frontend/src/home/ui/home-shortcuts.ts';

function press(init, target = document.body) {
  const event = new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init });
  target.dispatchEvent(event);
  return event;
}

describe('attachHomeShortcuts', () => {
  let onNewChat;
  let onToggleSearch;

  beforeEach(() => {
    onNewChat = vi.fn();
    onToggleSearch = vi.fn();
    attachHomeShortcuts({ onNewChat, onToggleSearch });
  });

  afterEach(() => detachHomeShortcuts());

  it('⌘O creates a new chat and prevents the default action', () => {
    const event = press({ key: 'o', metaKey: true });
    expect(onNewChat).toHaveBeenCalledTimes(1);
    expect(onToggleSearch).not.toHaveBeenCalled();
    expect(event.defaultPrevented).toBe(true);
  });

  it('⌘K toggles search and prevents the default action', () => {
    const event = press({ key: 'k', metaKey: true });
    expect(onToggleSearch).toHaveBeenCalledTimes(1);
    expect(onNewChat).not.toHaveBeenCalled();
    expect(event.defaultPrevented).toBe(true);
  });

  it('matches with Caps Lock on (uppercase key)', () => {
    press({ key: 'O', metaKey: true });
    press({ key: 'K', metaKey: true });
    expect(onNewChat).toHaveBeenCalledTimes(1);
    expect(onToggleSearch).toHaveBeenCalledTimes(1);
  });

  it('works while a text field has focus', () => {
    const input = document.createElement('input');
    document.body.appendChild(input);
    input.focus();
    press({ key: 'k', metaKey: true }, input);
    expect(onToggleSearch).toHaveBeenCalledTimes(1);
    input.remove();
  });

  it('ignores Ctrl, plain keys and other modifier combos', () => {
    press({ key: 'o', ctrlKey: true });
    press({ key: 'k', ctrlKey: true });
    press({ key: 'o' });
    press({ key: 'k' });
    press({ key: 'o', metaKey: true, shiftKey: true });
    press({ key: 'k', metaKey: true, altKey: true });
    press({ key: 'j', metaKey: true });
    expect(onNewChat).not.toHaveBeenCalled();
    expect(onToggleSearch).not.toHaveBeenCalled();
  });

  it('ignores key auto-repeat and IME composition', () => {
    const repeated = press({ key: 'o', metaKey: true, repeat: true });
    press({ key: 'k', metaKey: true, isComposing: true });
    expect(onNewChat).not.toHaveBeenCalled();
    expect(onToggleSearch).not.toHaveBeenCalled();
    expect(repeated.defaultPrevented).toBe(false);
  });

  it('detach removes the listener', () => {
    detachHomeShortcuts();
    const event = press({ key: 'o', metaKey: true });
    press({ key: 'k', metaKey: true });
    expect(onNewChat).not.toHaveBeenCalled();
    expect(onToggleSearch).not.toHaveBeenCalled();
    expect(event.defaultPrevented).toBe(false);
  });

  it('attaching again replaces the previous handlers', () => {
    const next = vi.fn();
    attachHomeShortcuts({ onNewChat: next, onToggleSearch });
    press({ key: 'o', metaKey: true });
    expect(onNewChat).not.toHaveBeenCalled();
    expect(next).toHaveBeenCalledTimes(1);
  });
});

describe('Home page shortcuts', () => {
  let container;
  let cleanup;
  let sessions;
  let createCalls;
  let releaseCreate;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    window.__TAURI__ = { event: { listen: vi.fn(async () => vi.fn()) } };
    sessions = [{ session_id: 's1', title: 'First chat' }];
    createCalls = 0;
    releaseCreate = null;
    vi.spyOn(api, 'createChannel').mockImplementation(async (onmessage) => ({ onmessage }));
    vi.spyOn(api, 'searchSpark').mockResolvedValue({ hits: [] });
    vi.spyOn(api, 'invoke').mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') return { sessions, current_session_id: sessions[0]?.session_id ?? '' };
      if (cmd === 'create_chat_session') {
        createCalls += 1;
        await new Promise((resolve) => {
          releaseCreate = resolve;
        });
        sessions = [{ session_id: `n${createCalls}`, title: 'New conversation' }, ...sessions];
        return { session_id: `n${createCalls}`, turns: [] };
      }
      return {};
    });
  });

  afterEach(() => {
    cleanup?.();
    vi.restoreAllMocks();
    container.remove();
    delete window.__TAURI__;
  });

  const modal = () => container.querySelector('[data-role="search-modal"]');
  const mount = async () => {
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
    await vi.waitFor(() => {
      expect(container.querySelectorAll('.home-chat-session').length).toBe(1);
    });
  };

  it('⌘K opens the search modal, and ⌘K again closes it', async () => {
    await mount();
    expect(modal()).toBeNull();
    press({ key: 'k', metaKey: true });
    await vi.waitFor(() => expect(modal()).not.toBeNull());
    press({ key: 'k', metaKey: true });
    await vi.waitFor(() => expect(modal()).toBeNull());
  });

  it('⌘K also works from inside the search input', async () => {
    await mount();
    press({ key: 'k', metaKey: true });
    await vi.waitFor(() => expect(modal()).not.toBeNull());
    press({ key: 'k', metaKey: true }, container.querySelector('[data-role="search-modal-input"]'));
    await vi.waitFor(() => expect(modal()).toBeNull());
  });

  it('⌘O creates a chat, switches to Spark AI, and closes an open search modal', async () => {
    await mount();
    press({ key: 'k', metaKey: true });
    await vi.waitFor(() => expect(modal()).not.toBeNull());
    press({ key: 'o', metaKey: true });
    await vi.waitFor(() => expect(createCalls).toBe(1));
    await vi.waitFor(() => expect(modal()).toBeNull());
    releaseCreate();
    await vi.waitFor(() => {
      expect(container.querySelector('.home-chat-pane').hidden).toBe(false);
    });
    expect(container.querySelector('.home-sidebar-tab[data-pane="chat"]').getAttribute('aria-selected')).toBe('true');
  });

  it('a second ⌘O while the first is still creating does not create another chat', async () => {
    await mount();
    press({ key: 'o', metaKey: true });
    await vi.waitFor(() => expect(createCalls).toBe(1));
    press({ key: 'o', metaKey: true });
    container.querySelector('[data-role="new-session"]').click();
    releaseCreate();
    await vi.waitFor(() => {
      expect(container.querySelector('.home-chat-pane').hidden).toBe(false);
    });
    expect(createCalls).toBe(1);
  });

  it('shows the bound shortcuts on the button hints', async () => {
    await mount();
    const dock = container.querySelector('[data-role="new-session"]');
    expect(dock.querySelector('kbd').textContent).toBe('⌘O');
    expect(dock.getAttribute('aria-keyshortcuts')).toBe('Meta+O');
    const search = container.querySelector('[data-role="sidebar-search"]');
    expect(search.getAttribute('data-tip')).toContain('⌘K');
    expect(search.getAttribute('aria-keyshortcuts')).toBe('Meta+K');
  });

  it('stops listening once the Home page unmounts', async () => {
    await mount();
    cleanup();
    cleanup = null;
    const event = press({ key: 'o', metaKey: true });
    press({ key: 'k', metaKey: true });
    expect(createCalls).toBe(0);
    expect(event.defaultPrevented).toBe(false);
    expect(modal()).toBeNull();
  });
});
