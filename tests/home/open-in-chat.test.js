// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { consumeComposerFocus } from '../../frontend/src/home/commands/composer-focus.ts';
import { openPathInChat } from '../../frontend/src/home/commands/open-in-chat.ts';
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
          staged: [{ id: 'F1', path: args.path, title: 'note' }],
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
    await openPathInChat('/tmp/notes/raw/note.md');

    expect(invokeSpy).toHaveBeenCalledWith('create_chat_session');
    expect(invokeSpy).toHaveBeenCalledWith('stage_chat_document', {
      sessionId: 's1',
      path: '/tmp/notes/raw/note.md',
    });
    expect(invokeSpy.mock.calls.some(([cmd]) => cmd === 'agent_chat_turn')).toBe(false);
    expect(navigateSpy).toHaveBeenCalledWith('#/home');
    expect(consumeComposerFocus()).toBe(true);
    expect(getHomeState().currentSessionId).toBe('s1');
    expect(getHomeState().staged).toEqual([
      { id: 'F1', path: '/tmp/notes/raw/note.md', title: 'note' },
    ]);
  });

  it('rejects an empty path before invoke', async () => {
    await expect(openPathInChat('   ')).rejects.toThrow('Missing file path');
    expect(invokeSpy).not.toHaveBeenCalled();
    expect(navigateSpy).not.toHaveBeenCalled();
  });
});
