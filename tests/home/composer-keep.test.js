// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import * as api from '../../frontend/src/host/api.ts';
import { HomePage } from '../../frontend/src/home/page.tsx';
import { requestComposerDraft, cancelComposerDraft } from '../../frontend/src/home/commands/composer-draft.ts';
import {
  loadComposerText,
  resetComposerKeep,
  saveComposerText,
} from '../../frontend/src/home/commands/composer-keep.ts';
import { stopHomeHub, resetHomeCommands } from '../../frontend/src/home/commands/hub.ts';
import { composerDisabled, composerValue, setComposerValue } from '../helpers/composer.js';

const NOTE_ID = 'f6692dc5d5242eec5206b6400e704bd6';
const NOTE = `[Agent Mock](note:${NOTE_ID})`;

describe('composer keep (store)', () => {
  beforeEach(resetComposerKeep);

  it('keeps the last saved text and starts empty', () => {
    expect(loadComposerText()).toBe('');
    saveComposerText('draft\nline');
    expect(loadComposerText()).toBe('draft\nline');
    resetComposerKeep();
    expect(loadComposerText()).toBe('');
  });
});

describe('composer text survives leaving and returning to home', () => {
  let container;
  let root;

  beforeEach(() => {
    resetComposerKeep();
    container = document.createElement('div');
    document.body.appendChild(container);
    window.__TAURI__ = { event: { listen: vi.fn(async () => vi.fn()) } };
    vi.spyOn(api, 'createChannel').mockImplementation(async (onmessage) => ({ onmessage }));
    vi.spyOn(api, 'invoke').mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') {
        return { sessions: [{ session_id: 's1', title: 'Chat' }], current_session_id: 's1' };
      }
      if (cmd === 'get_ai_assistant_binding') return { session_id: 's1', turns: [] };
      if (cmd === 'agent_chat_turn') return { reply_text: 'ok', terminal: 'ok' };
      return {};
    });
  });

  afterEach(() => {
    act(() => root?.unmount());
    stopHomeHub();
    resetHomeCommands();
    cancelComposerDraft();
    container.remove();
    delete window.__TAURI__;
    vi.restoreAllMocks();
  });

  /** The shell mounts HomePage on #/home and drops it on every other route. */
  async function mountHome() {
    root = createRoot(container);
    act(() => root.render(createElement(HomePage, { navigate: vi.fn() })));
    await vi.waitFor(() => {
      expect(composerDisabled(container.querySelector('[data-role="input"]'))).toBe(false);
    });
    return container.querySelector('[data-role="input"]');
  }

  function leaveHome() {
    act(() => root.unmount());
    root = null;
  }

  function type(input, text) {
    setComposerValue(input, text);
    act(() => {
      input.dispatchEvent(new Event('input', { bubbles: true }));
    });
  }

  it('restores typed text, including a chip, after coming back', async () => {
    let input = await mountHome();
    type(input, `read ${NOTE} now`);
    leaveHome();

    input = await mountHome();
    expect(composerValue(input)).toBe(`read ${NOTE} now`);
    expect(input.querySelectorAll('.home-ref-chip')).toHaveLength(1);
  });

  it('keeps a prefilled reference after jumping to the note and back', async () => {
    requestComposerDraft(`${NOTE} `);
    let input = await mountHome();
    await vi.waitFor(() => expect(composerValue(input)).toBe(`${NOTE} `));
    leaveHome();

    input = await mountHome();
    expect(composerValue(input)).toBe(`${NOTE} `);
    expect(input.querySelectorAll('.home-ref-chip')).toHaveLength(1);
  });

  it('does not bring back text that was sent', async () => {
    let input = await mountHome();
    type(input, 'hello');
    act(() => {
      input.dispatchEvent(
        new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }),
      );
    });
    await vi.waitFor(() => expect(composerValue(input)).toBe(''));
    leaveHome();

    input = await mountHome();
    expect(composerValue(input)).toBe('');
  });

  it('lets a new prefill replace what was kept', async () => {
    saveComposerText('old draft');
    requestComposerDraft('[New](note:' + NOTE_ID + ') ');
    const input = await mountHome();
    await vi.waitFor(() => expect(composerValue(input)).toBe(`[New](note:${NOTE_ID}) `));
  });
});
