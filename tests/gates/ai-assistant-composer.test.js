// @vitest-environment jsdom
/**
 * Home composer gate: Binding Contract query_binding (Present≠bound).
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import * as api from '../../frontend/js/host/api.js';
import { mountHomeHub } from '../../frontend/js/home-entry-shell/hub.js';

describe('Home chat composer Binding Contract gate', () => {
  let container;
  /** @type {import('vitest').MockInstance} */
  let invokeSpy;
  let listenHandlers;
  let bound;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    listenHandlers = {};
    bound = false;
    window.__TAURI__ = {
      event: {
        listen: vi.fn(async (name, handler) => {
          listenHandlers[name] = handler;
          return vi.fn();
        }),
      },
    };
    invokeSpy = vi.spyOn(api, 'invoke').mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: bound ? 'bound' : 'unbound' };
      if (cmd === 'list_chat_sessions') {
        return { sessions: [], current_session_id: '' };
      }
      if (cmd === 'get_ai_assistant_binding') {
        return { session_id: '', turns: [] };
      }
      return {};
    });
  });

  afterEach(() => {
    invokeSpy.mockRestore();
    container.remove();
    delete window.__TAURI__;
    vi.restoreAllMocks();
  });

  it('Present alone keeps the composer disabled', async () => {
    mountHomeHub(container, { navigate: vi.fn() });
    await vi.waitFor(() => {
      expect(invokeSpy).toHaveBeenCalledWith('query_binding');
    });
    expect(container.querySelector('[data-role="input"]').disabled).toBe(true);
    expect(container.querySelector('[data-role="send"]').disabled).toBe(true);
    expect(container.textContent).toMatch(/Chat requires a workspace Binding/);
    expect(container.textContent).not.toMatch(/No todo bound/);
  });

  it('binding-changed bound enables the composer', async () => {
    mountHomeHub(container, { navigate: vi.fn() });
    await vi.waitFor(() => {
      expect(listenHandlers['ai-assistant:binding-changed']).toBeTruthy();
    });
    bound = true;
    await listenHandlers['ai-assistant:binding-changed']({
      payload: { state: 'bound' },
    });
    await vi.waitFor(() => {
      expect(container.querySelector('[data-role="input"]').disabled).toBe(false);
      expect(container.querySelector('[data-role="send"]').disabled).toBe(false);
    });
  });

  it('binding-changed unbound disables the composer and clears the current session', async () => {
    bound = true;
    invokeSpy.mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: bound ? 'bound' : 'unbound' };
      if (cmd === 'list_chat_sessions') {
        return {
          sessions: [{ session_id: 's1', title: 'Hello from first' }],
          current_session_id: bound ? 's1' : '',
        };
      }
      if (cmd === 'get_ai_assistant_binding') {
        return {
          session_id: 's1',
          turns: [{ role: 'user', content: 'Hello from first' }],
        };
      }
      return {};
    });
    mountHomeHub(container, { navigate: vi.fn() });
    await vi.waitFor(() => {
      expect(container.querySelector('[data-session-id="s1"]')).not.toBeNull();
    });
    bound = false;
    await listenHandlers['ai-assistant:binding-changed']({
      payload: { state: 'unbound' },
    });
    await vi.waitFor(() => {
      expect(container.querySelector('[data-role="input"]').disabled).toBe(true);
      expect(container.textContent).toMatch(/Chat requires a workspace Binding/);
    });
  });
});
