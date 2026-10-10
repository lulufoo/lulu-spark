// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { consumeComposerDraft } from '../../frontend/src/home/commands/composer-draft.ts';
import { consumeComposerFocus } from '../../frontend/src/home/commands/composer-focus.ts';
import { openDraftInChat, openPathInChat } from '../../frontend/src/home/commands/open-in-chat.ts';
import { getHomeState, resetHomeState } from '../../frontend/src/home/state/store.ts';
import * as api from '../../frontend/src/host/api.ts';
import * as router from '../../frontend/src/router/index.ts';

describe('openPathInChat', () => {
  /** @type {import('vitest').MockInstance} */
  let invokeSpy;
  /** @type {import('vitest').MockInstance} */
  let navigateSpy;

  beforeEach(() => {
    resetHomeState();
    while (consumeComposerFocus()) {
      /* drain */
    }
    navigateSpy = vi.spyOn(router, 'navigate').mockImplementation(() => {});
    invokeSpy = vi.spyOn(api, 'invoke').mockImplementation(async (cmd, args) => {
      if (cmd === 'create_chat_session') {
        return { session_id: 's1' };
      }
      if (cmd === 'stage_chat_document') {
        return {
          session_id: 's1',
          id: 'F1',
          staged: [
            {
              id: 'F1',
              path: args.path,
              title: 'note',
              source: { kind: args.sourceKind, id: args.sourceId },
            },
          ],
        };
      }
      return {};
    });
  });

  afterEach(() => {
    invokeSpy.mockRestore();
    navigateSpy.mockRestore();
    resetHomeState();
    while (consumeComposerFocus()) {
      /* drain */
    }
  });

  it('stages the exact file, goes home, and does not send a message', async () => {
    await openPathInChat('/tmp/notes/raw/note.md', {
      kind: 'notes',
      id: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    });

    expect(invokeSpy).toHaveBeenCalledWith('create_chat_session');
    expect(invokeSpy).toHaveBeenCalledWith('stage_chat_document', {
      sessionId: 's1',
      path: '/tmp/notes/raw/note.md',
      sourceKind: 'notes',
      sourceId: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    });
    expect(invokeSpy.mock.calls.some(([cmd]) => cmd === 'agent_chat_turn')).toBe(false);
    expect(navigateSpy).toHaveBeenCalledWith('#/home');
    expect(consumeComposerFocus()).toBe(true);
    expect(getHomeState().currentSessionId).toBe('s1');
    expect(getHomeState().staged).toEqual([
      {
        id: 'F1',
        path: '/tmp/notes/raw/note.md',
        title: 'note',
        source: { kind: 'notes', id: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' },
      },
    ]);
  });

  it('rejects an empty path before invoke', async () => {
    await expect(
      openPathInChat('   ', { kind: 'notes', id: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' }),
    ).rejects.toThrow('Missing file path');
    expect(invokeSpy).not.toHaveBeenCalled();
    expect(navigateSpy).not.toHaveBeenCalled();
  });

  it('rejects a missing source id before invoke', async () => {
    await expect(openPathInChat('/tmp/notes/raw/note.md', { kind: 'notes', id: '' })).rejects.toThrow(
      'Missing source',
    );
    expect(invokeSpy).not.toHaveBeenCalled();
    expect(navigateSpy).not.toHaveBeenCalled();
  });
});

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
