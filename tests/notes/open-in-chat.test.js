// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { openNoteInChat } from '../../frontend/src/notes/commands/open-in-chat.ts';
import * as homeOpen from '../../frontend/src/home/commands/open-in-chat.ts';

describe('openNoteInChat', () => {
  /** @type {import('vitest').MockInstance} */
  let openSpy;

  beforeEach(() => {
    openSpy = vi.spyOn(homeOpen, 'openPathInChat').mockResolvedValue();
  });

  afterEach(() => {
    openSpy.mockRestore();
  });

  it('submits notes source from entry id', async () => {
    await openNoteInChat(
      { _id: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', common_path: 'inbox/note.md' },
      'en',
      'raw',
      '/spark',
    );
    expect(openSpy).toHaveBeenCalledWith('/spark/notes/raw/inbox/note.md', {
      kind: 'notes',
      id: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    });
  });

  it('rejects a missing notes archive id', async () => {
    await expect(
      openNoteInChat({ common_path: 'inbox/note.md' }, 'en', 'raw', '/spark'),
    ).rejects.toThrow('Missing source');
    expect(openSpy).not.toHaveBeenCalled();
  });
});
