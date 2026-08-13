// @vitest-environment jsdom
/**
 * Shell composer gate: Binding Contract query_binding (Present≠bound).
 * T5: discard cached sessionId on Unbound / new Bound; composer follows query_binding.
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { mountAiAssistant } from '../frontend/js/ai-assistant.js';

const appCss = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), '../frontend/app.css'),
  'utf8',
);

describe('ai-assistant composer Binding Contract gate', () => {
  let root;
  let invokeMock;
  let listenHandlers;
  /** @type {HTMLStyleElement | null} */
  let styleEl = null;

  beforeEach(() => {
    styleEl = document.createElement('style');
    styleEl.textContent = appCss;
    document.head.appendChild(styleEl);
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
    styleEl?.remove();
    styleEl = null;
    delete window.__TAURI__;
    vi.restoreAllMocks();
  });

  it('keeps [hidden] authoritative over flex display for Bound/Unbound panels', () => {
    expect(appCss).toMatch(
      /\.ai-assistant-panel\s+\.ai-assistant-(?:unbound|bound-content)\[hidden\]/,
    );
  });

  it('Present alone omits the binding status bar and keeps composer disabled', async () => {
    const api = mountAiAssistant(root);
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('query_binding');
    });
    const input = root.querySelector('[data-role="input"]');
    const send = root.querySelector('[data-role="send"]');
    expect(root.classList.contains('ai-assistant-panel')).toBe(true);
    expect(root.querySelector('[data-role="bound"]')).toBeNull();
    expect(root.querySelector('[data-role="unbound-content"]')?.hidden).toBe(false);
    expect(root.querySelector('[data-role="unbound-content"]')?.textContent).toContain('Unbound');
    expect(root.querySelector('[data-role="unbound-content"]')?.textContent).toContain(
      'No business context is currently bound.',
    );
    expect(root.querySelector('[data-role="unbound-content"]')?.textContent).not.toMatch(/todo/i);
    expect(root.querySelector('[data-role="bound-content"]')?.hidden).toBe(true);
    // display:flex must not override [hidden], or Unbound stays visible while Bound.
    expect(getComputedStyle(root.querySelector('[data-role="bound-content"]')).display).toBe(
      'none',
    );
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
    expect(root.querySelector('[data-role="bound"]')).toBeNull();
    expect(root.querySelector('[data-role="unbound-content"]')?.hidden).toBe(true);
    expect(root.querySelector('[data-role="bound-content"]')?.hidden).toBe(false);
    expect(getComputedStyle(root.querySelector('[data-role="unbound-content"]')).display).toBe(
      'none',
    );
    expect(getComputedStyle(root.querySelector('[data-role="bound-content"]')).display).not.toBe(
      'none',
    );
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
    expect(root.querySelector('[data-role="bound"]')).toBeNull();
    expect(root.querySelector('[data-role="unbound-content"]')?.hidden).toBe(false);
    expect(root.querySelector('[data-role="bound-content"]')?.hidden).toBe(true);
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
    expect(root.querySelector('[data-role="bound"]')).toBeNull();
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

  it('Enter submits the composer; Shift+Enter keeps the newline', async () => {
    invokeMock = vi.fn(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'get_ai_assistant_binding') {
        return { session_id: 'sess_enter_send', busy: false };
      }
      if (cmd === 'ensure_ai_assistant_session') {
        return { session_id: 'sess_enter_send', busy: false };
      }
      if (cmd === 'agent_chat_turn') {
        return { reply_text: 'ack', busy: false, terminal: 'ok' };
      }
      if (cmd === 'shell_close_ai_assistant') return { ok: true, state: 'unbound' };
      return {};
    });
    window.__TAURI__.core.invoke = invokeMock;

    const api = mountAiAssistant(root);
    await vi.waitFor(() => {
      expect(listenHandlers['ai-assistant:binding-changed']).toBeTruthy();
    });
    listenHandlers['ai-assistant:binding-changed']({
      payload: { state: 'bound' },
    });

    const input = root.querySelector('[data-role="input"]');
    expect(input.placeholder).toContain('Enter to send');
    expect(input.disabled).toBe(false);

    input.value = 'line one';
    input.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Enter', shiftKey: true, bubbles: true }),
    );
    expect(input.value).toBe('line one');
    expect(invokeMock).not.toHaveBeenCalledWith(
      'agent_chat_turn',
      expect.anything(),
    );

    input.value = 'hello via enter';
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        'agent_chat_turn',
        expect.objectContaining({
          sessionId: 'sess_enter_send',
          message: 'hello via enter',
          traceId: expect.stringMatching(/^ui_[A-Za-z0-9_-]{8,}$/),
        }),
      );
    });
    expect(input.value).toBe('');
    api.dispose();
  });

  it('ensures the current session before every turn, including a cached session', async () => {
    let ensureCount = 0;
    let turnCount = 0;
    invokeMock = vi.fn(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'get_ai_assistant_binding') {
        return { session_id: 'sess_cached', busy: false };
      }
      if (cmd === 'ensure_ai_assistant_session') {
        ensureCount += 1;
        return { session_id: 'sess_cached', busy: false };
      }
      if (cmd === 'agent_chat_turn') {
        turnCount += 1;
        return { reply_text: `ack-${turnCount}`, busy: false, terminal: 'none' };
      }
      if (cmd === 'shell_close_ai_assistant') return { ok: true, state: 'bound' };
      return {};
    });
    window.__TAURI__.core.invoke = invokeMock;

    const api = mountAiAssistant(root);
    await vi.waitFor(() => {
      expect(listenHandlers['ai-assistant:binding-changed']).toBeTruthy();
      expect(api.getState().hostBound).toBe(true);
      expect(api.getState().sessionId).toBe('sess_cached');
    });

    const input = root.querySelector('[data-role="input"]');
    for (const text of ['first turn', 'second turn']) {
      input.value = text;
      input.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
      await vi.waitFor(() => {
        expect(turnCount).toBe(text === 'first turn' ? 1 : 2);
      });
    }

    expect(ensureCount).toBe(2);
    const commands = invokeMock.mock.calls.map(([command]) => command);
    const ensureCalls = commands.reduce(
      (indices, command, index) =>
        command === 'ensure_ai_assistant_session' ? [...indices, index] : indices,
      [],
    );
    const turnCalls = commands.reduce(
      (indices, command, index) =>
        command === 'agent_chat_turn' ? [...indices, index] : indices,
      [],
    );
    expect(ensureCalls).toHaveLength(2);
    expect(turnCalls).toHaveLength(2);
    expect(ensureCalls[0]).toBeLessThan(turnCalls[0]);
    expect(ensureCalls[1]).toBeLessThan(turnCalls[1]);
    api.dispose();
  });
});
