// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  formatReferenceText,
  writeClipboardText,
} from '../../frontend/src/shared/reference-text.ts';

const ID = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

describe('formatReferenceText', () => {
  it('writes the kind, title and id on three lines', () => {
    expect(formatReferenceText({ kind: 'note', title: 'My note', id: ID })).toBe(
      `Lulu Spark note\ntitle: My note\nid: ${ID}`,
    );
  });

  it('names the knowledge kind', () => {
    expect(formatReferenceText({ kind: 'knowledge', title: 'Design', id: 'ab12' })).toBe(
      'Lulu Spark knowledge\ntitle: Design\nid: ab12',
    );
  });

  it('flattens newlines and repeated spaces in the title to one line', () => {
    expect(formatReferenceText({ kind: 'note', title: ' A\n  draft;\tv2 ', id: ID })).toBe(
      `Lulu Spark note\ntitle: A draft; v2\nid: ${ID}`,
    );
  });

  it('keeps no file path in the text', () => {
    expect(formatReferenceText({ kind: 'note', title: 'T', id: ID })).not.toContain('/');
  });
});

describe('writeClipboardText', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('writes the text and resolves true', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal('navigator', { clipboard: { writeText } });
    await expect(writeClipboardText('hello')).resolves.toBe(true);
    expect(writeText).toHaveBeenCalledWith('hello');
  });

  it('resolves false when the clipboard refuses', async () => {
    vi.stubGlobal('navigator', { clipboard: { writeText: vi.fn().mockRejectedValue(new Error('no')) } });
    await expect(writeClipboardText('hello')).resolves.toBe(false);
  });

  it('resolves false without a clipboard or without text', async () => {
    vi.stubGlobal('navigator', {});
    await expect(writeClipboardText('hello')).resolves.toBe(false);
    vi.stubGlobal('navigator', { clipboard: { writeText: vi.fn() } });
    await expect(writeClipboardText('')).resolves.toBe(false);
  });
});
