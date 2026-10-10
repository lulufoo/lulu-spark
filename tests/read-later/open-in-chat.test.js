// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  openReadLaterInChat,
  readLaterReferenceDraft,
} from '../../frontend/src/read-later/commands/open-in-chat.ts';
import * as homeOpen from '../../frontend/src/home/commands/open-in-chat.ts';
import { closeReadLaterDialog } from '../../frontend/src/read-later/commands/dialog.ts';
import { parseReferences } from '../../frontend/src/home/references/index.ts';

vi.mock('../../frontend/src/read-later/commands/dialog.ts', () => ({
  closeReadLaterDialog: vi.fn(),
}));

const ID = 'cccccccccccccccccccccccccccccccc';

describe('readLaterReferenceDraft', () => {
  it('links the title to the read-later id', () => {
    expect(readLaterReferenceDraft({ id: ID, url: 'https://example.com/a', title: 'Saved' })).toBe(
      `[Saved](read-later:${ID}) `,
    );
  });

  it('falls back to the url when the title is empty', () => {
    expect(readLaterReferenceDraft({ id: ID, url: 'https://example.com/a', title: '' })).toBe(
      `[https://example.com/a](read-later:${ID}) `,
    );
  });

  it('drops brackets and flattens whitespace so the link stays one chip', () => {
    expect(
      readLaterReferenceDraft({ id: ID, url: 'https://example.com/a', title: 'A [draft]\n  v2' }),
    ).toBe(`[A draft v2](read-later:${ID}) `);
  });

  it('is a reference the composer shows as a chip', () => {
    const [segment] = parseReferences(
      readLaterReferenceDraft({ id: ID, url: 'https://example.com/a', title: 'A [draft]' }),
    );
    expect(segment).toMatchObject({ type: 'ref', kind: 'read-later', id: ID, title: 'A draft' });
  });
});

describe('openReadLaterInChat', () => {
  /** @type {import('vitest').MockInstance} */
  let draftSpy;

  beforeEach(() => {
    draftSpy = vi.spyOn(homeOpen, 'openDraftInChat').mockResolvedValue();
    vi.mocked(closeReadLaterDialog).mockClear();
  });

  afterEach(() => {
    draftSpy.mockRestore();
  });

  it('closes the list dialog before opening the new chat', async () => {
    await openReadLaterInChat({ id: ID, url: 'https://example.com/a', title: 'Saved' });
    expect(closeReadLaterDialog).toHaveBeenCalledOnce();
    expect(draftSpy).toHaveBeenCalledWith(`[Saved](read-later:${ID}) `);
    expect(closeReadLaterDialog.mock.invocationCallOrder[0]).toBeLessThan(
      draftSpy.mock.invocationCallOrder[0],
    );
  });

  it('does not close the dialog when the entry has no id', async () => {
    await expect(openReadLaterInChat({ id: '', url: 'https://example.com/a' })).rejects.toThrow(
      'Missing source',
    );
    expect(closeReadLaterDialog).not.toHaveBeenCalled();
    expect(draftSpy).not.toHaveBeenCalled();
  });

  it('re-enables the button after opening, and after a failure', async () => {
    const button = document.createElement('button');
    await openReadLaterInChat({ id: ID, url: 'https://example.com/a', title: 'A' }, button);
    expect(button.disabled).toBe(false);

    draftSpy.mockRejectedValueOnce(new Error('boom'));
    await expect(
      openReadLaterInChat({ id: ID, url: 'https://example.com/a', title: 'A' }, button),
    ).rejects.toThrow('boom');
    expect(button.disabled).toBe(false);
  });
});
