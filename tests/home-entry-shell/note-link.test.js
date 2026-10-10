// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { marked } from 'marked';

vi.mock('../../frontend/src/host/api/note-open.ts', () => ({
  resolveNoteForOpen: vi.fn(),
}));
vi.mock('../../frontend/src/notes/commands/reload-index.ts', () => ({
  refreshNotesIndex: vi.fn(),
}));
vi.mock('../../frontend/src/home/commands/hub.ts', () => ({
  showActionError: vi.fn(),
}));

import { resolveNoteForOpen } from '../../frontend/src/host/api/note-open.ts';
import { refreshNotesIndex } from '../../frontend/src/notes/commands/reload-index.ts';
import { showActionError } from '../../frontend/src/home/commands/hub.ts';
import { state } from '../../frontend/src/host/state.ts';
import {
  noteIdFromHref,
  onNoteLinkClick,
} from '../../frontend/src/home/commands/open-note-link.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const ID = 'abcdefabcdefabcdefabcdefabcdefab';

function clickOn(html, selector) {
  const root = document.createElement('div');
  root.innerHTML = html;
  const event = { target: root.querySelector(selector), preventDefault: vi.fn() };
  onNoteLinkClick(event);
  return event;
}

function flush() {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

let opened;
const onOpen = (e) => opened.push(e.detail);

beforeEach(() => {
  opened = [];
  document.addEventListener('cta:open-entry', onOpen);
  vi.mocked(resolveNoteForOpen).mockReset();
  vi.mocked(refreshNotesIndex).mockReset();
  vi.mocked(showActionError).mockReset();
  state.index.data = { [ID]: { common_path: 'ai/note.md' } };
});

afterEach(() => {
  document.removeEventListener('cta:open-entry', onOpen);
  state.index.data = null;
});

describe('noteIdFromHref', () => {
  it('accepts note:<32 hex> only', () => {
    expect(noteIdFromHref(`note:${ID}`)).toBe(ID);
    expect(noteIdFromHref(`  note:${ID} `)).toBe(ID);
    for (const bad of ['note:bad', `note:${ID}x`, `https://x/${ID}`, `Note:${ID}`, '', null, undefined]) {
      expect(noteIdFromHref(bad)).toBeNull();
    }
  });
});

describe('onNoteLinkClick', () => {
  it('intercepts a note: link, resolves it in Rust, then opens the entry', async () => {
    vi.mocked(resolveNoteForOpen).mockResolvedValue({ id: ID, ok: true, common_path: 'ai/note.md' });
    const event = clickOn(`<p><a href="note:${ID}"><b>Title</b></a></p>`, 'b');
    expect(event.preventDefault).toHaveBeenCalledOnce();
    await flush();
    expect(resolveNoteForOpen).toHaveBeenCalledWith(ID);
    expect(opened).toEqual([{ common_path: 'ai/note.md' }]);
    expect(showActionError).not.toHaveBeenCalled();
  });

  it('leaves every other link alone', async () => {
    const event = clickOn('<a href="https://example.com">x</a>', 'a');
    expect(event.preventDefault).not.toHaveBeenCalled();
    const plain = clickOn('<span>no link</span>', 'span');
    expect(plain.preventDefault).not.toHaveBeenCalled();
    await flush();
    expect(resolveNoteForOpen).not.toHaveBeenCalled();
    expect(opened).toEqual([]);
  });

  it('reports a malformed note: link without calling Rust', async () => {
    const event = clickOn('<a href="note:bad">x</a>', 'a');
    expect(event.preventDefault).toHaveBeenCalledOnce();
    await flush();
    expect(resolveNoteForOpen).not.toHaveBeenCalled();
    expect(showActionError).toHaveBeenCalledWith({ message: expect.stringContaining('Cannot open note') });
    expect(opened).toEqual([]);
  });

  it('reports the Rust error when the id is unknown', async () => {
    vi.mocked(resolveNoteForOpen).mockRejectedValue(new Error('Entry not found'));
    clickOn(`<a href="note:${ID}">x</a>`, 'a');
    await flush();
    const err = vi.mocked(showActionError).mock.calls[0][0];
    expect(err.message).toBe('Cannot open note: Entry not found');
    expect(opened).toEqual([]);
  });

  it('refreshes a stale list once, then opens', async () => {
    state.index.data = {};
    vi.mocked(resolveNoteForOpen).mockResolvedValue({ id: ID, ok: true, common_path: 'ai/note.md' });
    vi.mocked(refreshNotesIndex).mockImplementation(async () => {
      state.index.data = { [ID]: { common_path: 'ai/note.md' } };
      return true;
    });
    clickOn(`<a href="note:${ID}">x</a>`, 'a');
    await flush();
    expect(refreshNotesIndex).toHaveBeenCalledOnce();
    expect(opened).toEqual([{ common_path: 'ai/note.md' }]);
  });

  it('reports instead of staying silent when the list still lacks the note', async () => {
    state.index.data = {};
    vi.mocked(resolveNoteForOpen).mockResolvedValue({ id: ID, ok: true, common_path: 'ai/note.md' });
    vi.mocked(refreshNotesIndex).mockResolvedValue(true);
    clickOn(`<a href="note:${ID}">x</a>`, 'a');
    await flush();
    expect(opened).toEqual([]);
    expect(vi.mocked(showActionError).mock.calls[0][0].message).toMatch(/Cannot open note/);
  });
});

describe('wiring', () => {
  it('marked keeps the note: href intact', () => {
    const html = marked.parse(`[Title](note:${ID})`);
    expect(html).toContain(`href="note:${ID}"`);
  });

  it('assistant markdown in the home page uses the click handler', () => {
    const page = readFileSync(join(repoRoot, 'frontend/src/home/page.tsx'), 'utf8');
    expect(page).toMatch(/onClick=\{onChatLinkClick\}/);
  });
});
