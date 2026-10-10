// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  noteReferenceDraft,
  openNoteInChat,
} from '../../frontend/src/notes/commands/open-in-chat.ts';
import * as homeOpen from '../../frontend/src/home/commands/open-in-chat.ts';
import { parseReferences } from '../../frontend/src/home/references/index.ts';

const ID = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

describe('noteReferenceDraft', () => {
  it('links the title to the note id', () => {
    expect(noteReferenceDraft({ _id: ID, common_path: 'inbox/x.md', title: 'My note' }, ID)).toBe(
      `[My note](note:${ID}) `,
    );
  });

  it('falls back to a title derived from the path', () => {
    expect(noteReferenceDraft({ _id: ID, common_path: 'inbox/my-note.md' }, ID)).toMatch(
      new RegExp(`^\\[.+\\]\\(note:${ID}\\) $`),
    );
  });

  it('drops brackets and flattens whitespace so the link stays one chip', () => {
    expect(
      noteReferenceDraft({ _id: ID, common_path: 'a.md', title: 'A [draft]\n  v2' }, ID),
    ).toBe(`[A draft v2](note:${ID}) `);
  });

  it('is a reference the composer shows as a chip', () => {
    const [segment] = parseReferences(
      noteReferenceDraft({ _id: ID, common_path: 'a.md', title: 'A [draft]' }, ID),
    );
    expect(segment).toMatchObject({ type: 'ref', kind: 'note', id: ID, title: 'A draft' });
  });
});

describe('openNoteInChat', () => {
  /** @type {import('vitest').MockInstance} */
  let draftSpy;

  beforeEach(() => {
    draftSpy = vi.spyOn(homeOpen, 'openDraftInChat').mockResolvedValue();
  });

  afterEach(() => {
    draftSpy.mockRestore();
  });

  it('opens a new chat prefilled with the note reference, not a staged file', async () => {
    await openNoteInChat({ _id: ID, common_path: 'inbox/note.md', title: 'Note' });
    expect(draftSpy).toHaveBeenCalledWith(`[Note](note:${ID}) `);
  });

  it('re-enables the button after opening, and after a failure', async () => {
    const button = document.createElement('button');
    await openNoteInChat({ _id: ID, common_path: 'a.md', title: 'A' }, button);
    expect(button.disabled).toBe(false);

    draftSpy.mockRejectedValueOnce(new Error('boom'));
    await expect(
      openNoteInChat({ _id: ID, common_path: 'a.md', title: 'A' }, button),
    ).rejects.toThrow('boom');
    expect(button.disabled).toBe(false);
  });

  it('rejects a missing notes archive id', async () => {
    await expect(openNoteInChat({ common_path: 'inbox/note.md' })).rejects.toThrow(
      'Missing source',
    );
    expect(draftSpy).not.toHaveBeenCalled();
  });
});
