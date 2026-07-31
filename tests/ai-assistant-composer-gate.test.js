// @vitest-environment jsdom
/**
 * Shell composer gate: Binding Contract query_binding (Present≠bound).
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { mountAiAssistant } from '../frontend/js/ai-assistant.js';

describe('ai-assistant composer Binding Contract gate', () => {
  let root;
  let invokeMock;
  let listenHandlers;

  beforeEach(() => {
    root = document.createElement('div');
    document.body.appendChild(root);
    listenHandlers = {};
    invokeMock = vi.fn(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'unbound' };
      if (cmd === 'get_ai_assistant_binding') {
        return { session_id: '', busy: false };
      }
      if (cmd === 'shell_close_ai_assistant') return { ok: true, state: 'unbound' };
      return {};
    });
    window.__TAURI__ = {
      core: { invoke: invokeMock },
      event: {
        listen: vi.fn(async (name, handler) => {
          listenHandlers[name] = handler;
          return vi.fn();
        }),
      },
    };
  });

  afterEach(() => {
    root?.remove();
    delete window.__TAURI__;
    vi.restoreAllMocks();
  });

  it('Present alone keeps Unbound and composer disabled', async () => {
    const api = mountAiAssistant(root);
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('query_binding');
    });
    const input = root.querySelector('[data-role="input"]');
    const send = root.querySelector('[data-role="send"]');
    const status = root.querySelector('[data-role="bound"]');
    expect(status.textContent).toBe('Unbound');
    expect(input.disabled).toBe(true);
    expect(send.disabled).toBe(true);
    expect(api.getState().hostBound).toBe(false);
    api.dispose();
  });

  it('binding-changed bound enables composer without requiring todo session copy', async () => {
    const api = mountAiAssistant(root);
    await vi.waitFor(() => {
      expect(listenHandlers['ai-assistant:binding-changed']).toBeTruthy();
    });
    listenHandlers['ai-assistant:binding-changed']({
      payload: { state: 'bound' },
    });
    const input = root.querySelector('[data-role="input"]');
    const send = root.querySelector('[data-role="send"]');
    const status = root.querySelector('[data-role="bound"]');
    expect(status.textContent).toBe('Bound');
    expect(input.disabled).toBe(false);
    expect(send.disabled).toBe(false);
    expect(api.getState().hostBound).toBe(true);
    expect(root.textContent).not.toMatch(/No todo bound/);
    api.dispose();
  });

  it('binding-changed unbound disables composer again', async () => {
    const api = mountAiAssistant(root);
    await vi.waitFor(() => {
      expect(listenHandlers['ai-assistant:binding-changed']).toBeTruthy();
    });
    listenHandlers['ai-assistant:binding-changed']({
      payload: { state: 'bound' },
    });
    listenHandlers['ai-assistant:binding-changed']({
      payload: { state: 'unbound' },
    });
    expect(root.querySelector('[data-role="bound"]').textContent).toBe('Unbound');
    expect(root.querySelector('[data-role="input"]').disabled).toBe(true);
    expect(api.getState().hostBound).toBe(false);
    api.dispose();
  });
});
