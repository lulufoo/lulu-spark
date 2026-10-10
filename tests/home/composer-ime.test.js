// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import * as api from '../../frontend/src/host/api.ts';
import { mountHomeHub } from '../../frontend/src/home/hub.tsx';
import { setComposerValue, composerDisabled } from '../helpers/composer.js';

function enterKey(init = {}) {
  const event = new KeyboardEvent('keydown', {
    key: 'Enter',
    bubbles: true,
    cancelable: true,
    ...init,
  });
  if (init.keyCode != null) {
    Object.defineProperty(event, 'keyCode', { get: () => init.keyCode });
  }
  return event;
}

describe('home composer IME Enter', () => {
  let container;
  let cleanup;
  /** @type {import('vitest').MockInstance} */
  let invokeSpy;
  /** @type {import('vitest').MockInstance} */
  let createChannelSpy;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    cleanup = null;
    window.__TAURI__ = {
      event: {
        listen: vi.fn(async () => vi.fn()),
      },
    };
    createChannelSpy = vi.spyOn(api, 'createChannel').mockImplementation(async (onmessage) => ({
      onmessage,
    }));
    invokeSpy = vi.spyOn(api, 'invoke').mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') {
        return {
          sessions: [{ session_id: 's1', title: 'Chat' }],
          current_session_id: 's1',
        };
      }
      if (cmd === 'get_ai_assistant_binding') {
        return { session_id: 's1', turns: [] };
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
    vi.restoreAllMocks();
  });

  async function mountReady() {
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
    await vi.waitFor(() => {
      expect(composerDisabled(container.querySelector('[data-role=\"input\"]'))).toBe(false);
    });
  }

  it('does not send on the WebKit confirming Enter after compositionend', async () => {
    await mountReady();
    const input = container.querySelector('[data-role="input"]');
    setComposerValue(input, 'nihao');
    input.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
    input.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true }));
    input.dispatchEvent(enterKey({ isComposing: false }));
    await Promise.resolve();
    expect(invokeSpy).not.toHaveBeenCalledWith('agent_chat_turn', expect.anything());
  });

  it('sends on Enter after the IME reset tick', async () => {
    await mountReady();
    const input = container.querySelector('[data-role="input"]');
    setComposerValue(input, '你好');
    input.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
    input.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true }));
    await new Promise((resolve) => {
      setTimeout(resolve, 0);
    });
    input.dispatchEvent(enterKey({ isComposing: false }));
    await vi.waitFor(() => {
      expect(invokeSpy).toHaveBeenCalledWith(
        'agent_chat_turn',
        expect.objectContaining({ sessionId: 's1', message: '你好' }),
      );
    });
  });
});
