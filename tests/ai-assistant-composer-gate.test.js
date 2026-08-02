// @vitest-environment jsdom
/**
 * Shell composer gate: Binding Contract query_binding (Present≠bound).
 * T5: discard cached sessionId on Unbound / new Bound; composer follows query_binding.
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

  it('binding-changed unbound discards cached sessionId', async () => {
    const api = mountAiAssistant(root);
    await vi.waitFor(() => {
      expect(listenHandlers['ai-assistant:binding-changed']).toBeTruthy();
    });
    api.applyBinding({ session_id: 'sess_cached_old', busy: false });
    expect(api.getState().sessionId).toBe('sess_cached_old');
    listenHandlers['ai-assistant:binding-changed']({
      payload: { state: 'bound' },
    });
    listenHandlers['ai-assistant:binding-changed']({
      payload: { state: 'unbound' },
    });
    expect(api.getState().hostBound).toBe(false);
    expect(api.getState().sessionId).toBe('');
    expect(root.querySelector('[data-role="input"]').disabled).toBe(true);
    api.dispose();
  });

  it('binding-changed new Bound discards cached sessionId; composer follows query_binding', async () => {
    const api = mountAiAssistant(root);
    await vi.waitFor(() => {
      expect(listenHandlers['ai-assistant:binding-changed']).toBeTruthy();
    });
    api.applyBinding({ session_id: 'sess_stale_before_rebind', busy: false });
    expect(api.getState().sessionId).toBe('sess_stale_before_rebind');
    listenHandlers['ai-assistant:binding-changed']({
      payload: { state: 'bound' },
    });
    expect(api.getState().hostBound).toBe(true);
    expect(api.getState().sessionId).toBe('');
    expect(root.querySelector('[data-role="bound"]').textContent).toBe('Bound');
    expect(root.querySelector('[data-role="input"]').disabled).toBe(false);
    api.dispose();
  });

  // SK-3 / T5: turns hydrate on mount/re-show; dispose ≠ Reset / ≠ switch live.
  it('hydrates bubbles from binding.turns on mount pull', async () => {
    invokeMock = vi.fn(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'get_ai_assistant_binding') {
        return {
          session_id: 'sess_live_hydrate',
          busy: false,
          turns: [
            { role: 'user', content: 'prior user' },
            { role: 'assistant', content: 'prior assistant' },
          ],
        };
      }
      if (cmd === 'shell_close_ai_assistant') return { ok: true, state: 'bound' };
      return {};
    });
    window.__TAURI__.core.invoke = invokeMock;

    const api = mountAiAssistant(root);
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('get_ai_assistant_binding');
    });
    await vi.waitFor(() => {
      expect(root.textContent).toContain('prior user');
      expect(root.textContent).toContain('prior assistant');
    });
    expect(api.getState().sessionId).toBe('sess_live_hydrate');
    expect(api.getState().messages.map((m) => m.text)).toEqual([
      'prior user',
      'prior assistant',
    ]);
    api.dispose();
  });

  it('remount after dispose re-hydrates history; dispose never Reset/切 live', async () => {
    invokeMock = vi.fn(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'get_ai_assistant_binding') {
        return {
          session_id: 'sess_keep_live',
          busy: false,
          turns: [
            { role: 'user', content: 'kept across close' },
            { role: 'assistant', content: 'still here' },
          ],
        };
      }
      if (cmd === 'shell_close_ai_assistant') return { ok: true, state: 'bound' };
      if (cmd === 'reset_binding') return { ok: true, state: 'unbound' };
      return {};
    });
    window.__TAURI__.core.invoke = invokeMock;

    const first = mountAiAssistant(root);
    await vi.waitFor(() => {
      expect(root.textContent).toContain('kept across close');
    });
    first.dispose();
    expect(invokeMock).toHaveBeenCalledWith('shell_close_ai_assistant');
    expect(invokeMock).not.toHaveBeenCalledWith('reset_binding');
    expect(invokeMock).not.toHaveBeenCalledWith(
      'reset_binding',
      expect.anything(),
    );

    const second = mountAiAssistant(root);
    await vi.waitFor(() => {
      expect(root.textContent).toContain('kept across close');
      expect(root.textContent).toContain('still here');
    });
    expect(second.getState().sessionId).toBe('sess_keep_live');
    second.dispose();
  });

  it('empty turns after Reset leaves no executable history bubbles', async () => {
    invokeMock = vi.fn(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'unbound' };
      if (cmd === 'get_ai_assistant_binding') {
        return { session_id: '', busy: false, turns: [] };
      }
      if (cmd === 'shell_close_ai_assistant') return { ok: true, state: 'unbound' };
      return {};
    });
    window.__TAURI__.core.invoke = invokeMock;

    const api = mountAiAssistant(root);
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('get_ai_assistant_binding');
    });
    expect(api.getState().messages).toEqual([]);
    expect(root.querySelectorAll('.ai-assistant-bubble').length).toBe(0);
    api.dispose();
  });
});
