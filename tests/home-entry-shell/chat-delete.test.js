// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';
import { mountHomeHub } from '../../frontend/src/home/hub.tsx';
import { neighborSessionId } from '../../frontend/src/home/state/store.ts';
import { setComposerValue } from '../helpers/composer.js';

describe('neighborSessionId', () => {
  it('picks the next row, else the previous, else empty', () => {
    const rows = [{ session_id: 'a' }, { session_id: 'b' }, { session_id: 'c' }];
    expect(neighborSessionId(rows, 'a')).toBe('b');
    expect(neighborSessionId(rows, 'b')).toBe('c');
    expect(neighborSessionId(rows, 'c')).toBe('b');
    expect(neighborSessionId([{ session_id: 'a' }], 'a')).toBe('');
  });
});

describe('home chat delete conversation', () => {
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
    sessions = [
      { session_id: 's1', title: 'Hello from first' },
      { session_id: 's2', title: 'New conversation' },
    ];
    currentId = 's2';
    window.__TAURI__ = {
      event: {
        listen: vi.fn(async () => vi.fn()),
      },
    };
    createChannelSpy = vi.spyOn(api, 'createChannel').mockImplementation(async (onmessage) => ({
      onmessage,
    }));
    invokeSpy = vi.spyOn(api, 'invoke').mockImplementation(async (cmd, args) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') {
        return { sessions, current_session_id: currentId };
      }
      if (cmd === 'get_ai_assistant_binding' || cmd === 'select_chat_session') {
        if (cmd === 'select_chat_session') currentId = String(args?.sessionId || '');
        return {
          session_id: currentId,
          turns:
            currentId === 's1' ? [{ role: 'user', content: 'Hello from first' }] : [],
        };
      }
      if (cmd === 'delete_chat_session') {
        const id = String(args?.sessionId || '');
        sessions = sessions.filter((s) => s.session_id !== id);
        if (currentId === id) currentId = '';
        return { sessions, current_session_id: currentId };
      }
      if (cmd === 'agent_chat_turn') {
        return { reply_text: 'Hi back', terminal: 'ok' };
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

  it('asks for confirm and does not delete on cancel', async () => {
    await mountReady();
    container.querySelector('[data-role="delete-session"][data-session-id="s1"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('[data-role="delete-session-confirm"]')).not.toBeNull();
    });
    expect(container.textContent).toMatch(/Delete conversation/);
    expect(container.textContent).toMatch(/cannot be undone/);
    container.querySelector('[data-role="cancel-delete-session"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('[data-role="delete-session-confirm"]')).toBeNull();
    });
    expect(invokeSpy).not.toHaveBeenCalledWith(
      'delete_chat_session',
      expect.anything(),
    );
    expect(container.querySelector('[data-session-id="s1"]')).not.toBeNull();
  });

  it('deletes the current session after confirm and selects the neighbor', async () => {
    await mountReady();
    container.querySelector('[data-role="delete-session"][data-session-id="s2"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('[data-role="confirm-delete-session"]')).not.toBeNull();
    });
    container.querySelector('[data-role="confirm-delete-session"]').click();
    await vi.waitFor(() => {
      expect(invokeSpy).toHaveBeenCalledWith('delete_chat_session', { sessionId: 's2' });
      expect(invokeSpy).toHaveBeenCalledWith('select_chat_session', { sessionId: 's1' });
      expect(container.querySelector('[data-session-id="s2"]')).toBeNull();
      expect(
        container.querySelector('[data-session-id="s1"]')?.closest('.home-chat-session')
          ?.classList.contains('is-active'),
      ).toBe(true);
    });
  });

  it('hides delete while the session is in flight', async () => {
    let releaseTurn;
    const turnGate = new Promise((resolve) => {
      releaseTurn = resolve;
    });
    invokeSpy.mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') {
        return { sessions, current_session_id: currentId };
      }
      if (cmd === 'get_ai_assistant_binding') {
        return { session_id: currentId, turns: [] };
      }
      if (cmd === 'agent_chat_turn') {
        await turnGate;
        return { reply_text: 'Hi back', terminal: 'ok' };
      }
      return {};
    });
    await mountReady();
    const input = container.querySelector('[data-role="input"]');
    const form = container.querySelector('[data-role="form"]');
    setComposerValue(input, 'Hello there');
    form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    await vi.waitFor(() => {
      expect(
        container.querySelector('[data-role="delete-session"][data-session-id="s2"]'),
      ).toBeNull();
      expect(
        container.querySelector('[data-role="delete-session"][data-session-id="s1"]'),
      ).not.toBeNull();
    });
    releaseTurn();
  });
});
