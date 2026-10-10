// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { consumeComposerDraft } from '../../frontend/src/home/commands/composer-draft.ts';
import { consumeComposerFocus } from '../../frontend/src/home/commands/composer-focus.ts';
import { openDraftInChat } from '../../frontend/src/home/commands/open-in-chat.ts';
import { getHomeState, resetHomeState } from '../../frontend/src/home/state/store.ts';
import * as api from '../../frontend/src/host/api.ts';
import * as router from '../../frontend/src/router/index.ts';

describe('openDraftInChat', () => {
  /** @type {import('vitest').MockInstance} */
  let invokeSpy;
  /** @type {import('vitest').MockInstance} */
  let navigateSpy;

  beforeEach(() => {
    resetHomeState();
    consumeComposerDraft();
    while (consumeComposerFocus()) {
      /* drain */
    }
    navigateSpy = vi.spyOn(router, 'navigate').mockImplementation(() => {});
    invokeSpy = vi.spyOn(api, 'invoke').mockImplementation(async (cmd) => {
      if (cmd === 'create_chat_session') return { session_id: 's1' };
      return {};
    });
  });

  afterEach(() => {
    invokeSpy.mockRestore();
    navigateSpy.mockRestore();
    resetHomeState();
    consumeComposerDraft();
    while (consumeComposerFocus()) {
      /* drain */
    }
  });

  it('opens a new session with the draft pending; stages and sends nothing', async () => {
    await openDraftInChat('[N](note:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa)');

    expect(invokeSpy).toHaveBeenCalledWith('create_chat_session');
    const cmds = invokeSpy.mock.calls.map(([cmd]) => cmd);
    expect(cmds).not.toContain('stage_chat_document');
    expect(cmds).not.toContain('agent_chat_turn');
    expect(getHomeState().currentSessionId).toBe('s1');
    expect(navigateSpy).toHaveBeenCalledWith('#/home');
    expect(consumeComposerFocus()).toBe(true);
    expect(consumeComposerDraft()).toBe('[N](note:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa)');
  });

  it('has the draft pending before the session id lands in state', async () => {
    let draftAtSessionSwitch = null;
    invokeSpy.mockImplementation(async (cmd) => {
      if (cmd === 'create_chat_session') {
        draftAtSessionSwitch = consumeComposerFocus();
        return { session_id: 's1' };
      }
      return {};
    });

    await openDraftInChat('hello');

    expect(draftAtSessionSwitch).toBe(true);
  });

  it('drops the pending draft and does not navigate when session creation fails', async () => {
    invokeSpy.mockImplementation(async (cmd) => {
      if (cmd === 'create_chat_session') throw new Error('no session');
      return {};
    });

    await expect(openDraftInChat('hello')).rejects.toThrow('no session');

    expect(navigateSpy).not.toHaveBeenCalled();
    expect(consumeComposerDraft()).toBeNull();
    expect(consumeComposerFocus()).toBe(false);
  });

  it('rejects an empty draft before invoke', async () => {
    await expect(openDraftInChat('   ')).rejects.toThrow('Missing draft');
    expect(invokeSpy).not.toHaveBeenCalled();
    expect(navigateSpy).not.toHaveBeenCalled();
  });
});
