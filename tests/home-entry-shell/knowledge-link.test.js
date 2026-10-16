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
import { state } from '../../frontend/src/host/state.ts';
import { showActionError } from '../../frontend/src/home/commands/hub.ts';
import {
  knowledgeIdFromHref,
  onKnowledgeLinkClick,
} from '../../frontend/src/home/commands/open-knowledge-link.ts';
import { onChatLinkClick } from '../../frontend/src/home/commands/open-chat-link.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const ID = 'abcdefabcdefabcdefabcdefabcdefab';
const LEGACY_ID = '0123456789ab';

function clickOn(handler, html, selector) {
  const root = document.createElement('div');
  root.innerHTML = html;
  const event = { target: root.querySelector(selector), preventDefault: vi.fn() };
  handler(event);
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
  vi.mocked(showActionError).mockReset();
  state.index.data = { [ID]: { folder: 'guide', id: ID }, [LEGACY_ID]: { folder: 'guide', id: LEGACY_ID } };
});

afterEach(() => {
  document.removeEventListener('cta:open-entry', onOpen);
  state.index.data = null;
});

describe('knowledgeIdFromHref', () => {
  it('accepts knowledge:<hex> of any length', () => {
    expect(knowledgeIdFromHref(`knowledge:${ID}`)).toBe(ID);
    expect(knowledgeIdFromHref(`knowledge:${LEGACY_ID}`)).toBe(LEGACY_ID);
    expect(knowledgeIdFromHref(`  knowledge:${ID} `)).toBe(ID);
  });

  it('rejects anything that is not a knowledge: hex link', () => {
    for (const bad of ['knowledge:', 'knowledge:xyz', `knowledge:${ID}/x`, `note:${ID}`, `Knowledge:${ID}`, '', null, undefined]) {
      expect(knowledgeIdFromHref(bad)).toBeNull();
    }
  });
});

describe('onKnowledgeLinkClick', () => {
  it('intercepts a knowledge: link and opens it as a workspace document', async () => {
    vi.mocked(resolveNoteForOpen).mockResolvedValue({ id: ID, ok: true, workspace: 'default' });
    const event = clickOn(onKnowledgeLinkClick, `<p><a href="knowledge:${ID}"><b>Title</b></a></p>`, 'b');
    expect(event.preventDefault).toHaveBeenCalledOnce();
    await flush();
    expect(resolveNoteForOpen).toHaveBeenCalledWith(ID);
    expect(opened).toEqual([{ id: ID }]);
    expect(showActionError).not.toHaveBeenCalled();
  });

  it('opens a legacy 12-char id the same way', async () => {
    vi.mocked(resolveNoteForOpen).mockResolvedValue({ id: LEGACY_ID, ok: true, workspace: 'default' });
    clickOn(onKnowledgeLinkClick, `<a href="knowledge:${LEGACY_ID}">x</a>`, 'a');
    await flush();
    expect(resolveNoteForOpen).toHaveBeenCalledWith(LEGACY_ID);
    expect(opened).toEqual([{ id: LEGACY_ID }]);
  });

  it('leaves every other link alone', async () => {
    const web = clickOn(onKnowledgeLinkClick, '<a href="https://example.com">x</a>', 'a');
    const note = clickOn(onKnowledgeLinkClick, `<a href="note:${ID}">x</a>`, 'a');
    const plain = clickOn(onKnowledgeLinkClick, '<span>no link</span>', 'span');
    for (const event of [web, note, plain]) expect(event.preventDefault).not.toHaveBeenCalled();
    await flush();
    expect(resolveNoteForOpen).not.toHaveBeenCalled();
    expect(opened).toEqual([]);
  });

  it('reports a malformed knowledge: link without calling Rust', async () => {
    const event = clickOn(onKnowledgeLinkClick, '<a href="knowledge:zzz">x</a>', 'a');
    expect(event.preventDefault).toHaveBeenCalledOnce();
    await flush();
    expect(resolveNoteForOpen).not.toHaveBeenCalled();
    expect(vi.mocked(showActionError).mock.calls[0][0].message).toMatch(/Cannot open knowledge document/);
    expect(opened).toEqual([]);
  });

  it('reports the Rust error when the id is unknown', async () => {
    vi.mocked(resolveNoteForOpen).mockRejectedValue(new Error('Unknown document id'));
    clickOn(onKnowledgeLinkClick, `<a href="knowledge:${ID}">x</a>`, 'a');
    await flush();
    expect(vi.mocked(showActionError).mock.calls[0][0].message).toBe('Cannot open note: Unknown document id');
    expect(opened).toEqual([]);
  });
});

describe('onChatLinkClick', () => {
  it('routes knowledge: links through the shared document opener', async () => {
    vi.mocked(resolveNoteForOpen).mockResolvedValue({ id: ID, ok: true, workspace: 'default' });
    clickOn(onChatLinkClick, `<a href="knowledge:${ID}">x</a>`, 'a');
    await flush();
    expect(resolveNoteForOpen).toHaveBeenCalledWith(ID);
    expect(opened).toEqual([{ id: ID }]);
  });

  it('keeps routing note: links to the notes flow', async () => {
    vi.mocked(resolveNoteForOpen).mockRejectedValue(new Error('Entry not found'));
    clickOn(onChatLinkClick, `<a href="note:${ID}">x</a>`, 'a');
    await flush();
    expect(resolveNoteForOpen).toHaveBeenCalledWith(ID);
  });
});

describe('wiring', () => {
  it('marked keeps the knowledge: href intact', () => {
    expect(marked.parse(`[Title](knowledge:${ID})`)).toContain(`href="knowledge:${ID}"`);
  });

  it('assistant markdown in the home page uses the shared chat link handler', () => {
    const page = readFileSync(join(repoRoot, 'frontend/src/home/page.tsx'), 'utf8');
    expect(page).toMatch(/onClick=\{onChatLinkClick\}/);
  });
});
