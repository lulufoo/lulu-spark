// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  copyNoteReference,
  noteReferenceDraft,
  openNoteInChat,
} from '../../frontend/src/notes/commands/open-in-chat.ts';
import * as homeOpen from '../../frontend/src/home/commands/open-in-chat.ts';
import { parseReferences } from '../../frontend/src/home/references/index.ts';
import { state } from '../../frontend/src/host/state.ts';

const ID = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

const DEFAULT_WORKSPACE = { id: 'default', name: 'Default', view_mode: 'directory', is_default: true };

function resetTitleSources() {
  state.workspaces = { current: 'default', list: [DEFAULT_WORKSPACE] };
  state.index.titleCache = new Map();
  state.viewer.entry = null;
  state.viewer.rawText = '';
}

describe('noteReferenceDraft', () => {
  beforeEach(resetTitleSources);
  afterEach(resetTitleSources);

  it('links the title to the note id', () => {
    expect(noteReferenceDraft({ _id: ID, common_path: 'inbox/x.md', title: 'My note' }, ID)).toBe(
      `[Default：My note](doc:${ID}) `,
    );
  });

  it('uses the card title cache when the index has no title field', () => {
    const entry = {
      _id: ID,
      common_path: 'inbox/notes/202610110218-abc123.md',
      created_at: '202610110218',
    };
    state.index.titleCache.set('20261011', new Map([[ID, 'Real Title']]));
    expect(noteReferenceDraft(entry, ID)).toBe(`[Default：Real Title](doc:${ID}) `);
  });

  it('uses the open viewer H1 when the card cache is empty', () => {
    const entry = {
      _id: ID,
      common_path: 'inbox/notes/202610110218-abc123.md',
      created_at: '202610110218',
    };
    state.viewer.entry = entry;
    state.viewer.rawText = '# Viewer Title\n\nbody';
    expect(noteReferenceDraft(entry, ID)).toBe(`[Default：Viewer Title](doc:${ID}) `);
  });

  it('falls back to Untitled note when no title is available', () => {
    expect(noteReferenceDraft({ _id: ID, folder: 'inbox' }, ID)).toBe(
      `[Default：Untitled note](doc:${ID}) `,
    );
  });

  it('drops brackets and flattens whitespace so the link stays one chip', () => {
    expect(
      noteReferenceDraft({ _id: ID, common_path: 'a.md', title: 'A [draft]\n  v2' }, ID),
    ).toBe(`[Default：A draft v2](doc:${ID}) `);
  });

  it('shows the current name of the entry workspace, so a rename shows up at once', () => {
    state.workspaces = {
      current: 'default',
      list: [DEFAULT_WORKSPACE, { id: 'k3x9ab', name: 'Research', view_mode: 'timeline', is_default: false }],
    };
    const entry = { _id: ID, workspace: 'k3x9ab', common_path: 'a.md', title: 'Paper' };
    expect(noteReferenceDraft(entry, ID)).toBe(`[Research：Paper](doc:${ID}) `);
    expect(noteReferenceDraft({ ...entry, workspace: 'gone' }, ID)).toBe(`[gone：Paper](doc:${ID}) `);
  });

  it('is a reference the composer shows as a chip', () => {
    const [segment] = parseReferences(
      noteReferenceDraft({ _id: ID, common_path: 'a.md', title: 'A [draft]' }, ID),
    );
    expect(segment).toMatchObject({ type: 'ref', kind: 'doc', id: ID, title: 'Default：A draft' });
  });
});

describe('openNoteInChat', () => {
  /** @type {import('vitest').MockInstance} */
  let draftSpy;

  beforeEach(() => {
    resetTitleSources();
    draftSpy = vi.spyOn(homeOpen, 'openDraftInChat').mockResolvedValue();
  });

  afterEach(() => {
    draftSpy.mockRestore();
    resetTitleSources();
  });

  it('opens a new chat prefilled with the note reference, not a staged file', async () => {
    await openNoteInChat({ _id: ID, common_path: 'inbox/note.md', title: 'Note' });
    expect(draftSpy).toHaveBeenCalledWith(`[Default：Note](doc:${ID}) `);
  });

  it('prefills the card title when the index entry has no title field', async () => {
    const entry = {
      _id: ID,
      common_path: 'inbox/notes/202610110218-abc123.md',
      created_at: '202610110218',
    };
    state.index.titleCache.set('20261011', new Map([[ID, 'Real Title']]));
    await openNoteInChat(entry);
    expect(draftSpy).toHaveBeenCalledWith(`[Default：Real Title](doc:${ID}) `);
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

describe('copyNoteReference', () => {
  /** @type {import('vitest').Mock} */
  let writeText;

  beforeEach(() => {
    resetTitleSources();
    writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal('navigator', { clipboard: { writeText } });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    resetTitleSources();
  });

  it('copies the kind, title and id, with no file path', async () => {
    const copied = await copyNoteReference({
      _id: ID,
      common_path: 'inbox/notes/202610110218-abc123.md',
      title: 'My note',
    });
    expect(copied).toBe(true);
    expect(writeText).toHaveBeenCalledWith(`Lulu Spark doc\ntitle: My note\nid: ${ID}`);
    expect(writeText.mock.calls[0][0]).not.toContain('inbox/');
  });

  it('uses the same title the chat reference uses', async () => {
    const entry = {
      _id: ID,
      common_path: 'inbox/notes/202610110218-abc123.md',
      created_at: '202610110218',
    };
    state.index.titleCache.set('20261011', new Map([[ID, 'Real Title']]));
    await copyNoteReference(entry);
    expect(writeText).toHaveBeenCalledWith(`Lulu Spark doc\ntitle: Real Title\nid: ${ID}`);
  });

  it('copies nothing for a missing entry or a missing archive id', async () => {
    await expect(copyNoteReference(null)).resolves.toBe(false);
    await expect(copyNoteReference({ common_path: 'inbox/note.md' })).resolves.toBe(false);
    expect(writeText).not.toHaveBeenCalled();
  });
});
