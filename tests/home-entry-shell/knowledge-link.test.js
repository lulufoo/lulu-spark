// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { marked } from 'marked';

vi.mock('../../frontend/src/host/api/knowledge-open.ts', () => ({
  resolveKnowledgeForOpen: vi.fn(),
}));
vi.mock('../../frontend/src/host/api/note-open.ts', () => ({
  resolveNoteForOpen: vi.fn(),
}));
vi.mock('../../frontend/src/notes/commands/reload-index.ts', () => ({
  refreshNotesIndex: vi.fn(),
}));
vi.mock('../../frontend/src/home/commands/hub.ts', () => ({
  showActionError: vi.fn(),
}));

import { resolveKnowledgeForOpen } from '../../frontend/src/host/api/knowledge-open.ts';
import { resolveNoteForOpen } from '../../frontend/src/host/api/note-open.ts';
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
  document.addEventListener('cta:open-kb-doc', onOpen);
  vi.mocked(resolveKnowledgeForOpen).mockReset();
  vi.mocked(resolveNoteForOpen).mockReset();
  vi.mocked(showActionError).mockReset();
});

afterEach(() => {
  document.removeEventListener('cta:open-kb-doc', onOpen);
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
  it('intercepts a knowledge: link, resolves it in Rust, then opens the document', async () => {
    vi.mocked(resolveKnowledgeForOpen).mockResolvedValue({ id: ID, ok: true, repo: 'demo', path: 'guide/intro.md' });
    const event = clickOn(onKnowledgeLinkClick, `<p><a href="knowledge:${ID}"><b>Title</b></a></p>`, 'b');
    expect(event.preventDefault).toHaveBeenCalledOnce();
    await flush();
    expect(resolveKnowledgeForOpen).toHaveBeenCalledWith(ID);
    expect(opened).toEqual([{ repo: 'demo', path: 'guide/intro.md' }]);
    expect(showActionError).not.toHaveBeenCalled();
  });

  it('opens a legacy 12-char id the same way', async () => {
    vi.mocked(resolveKnowledgeForOpen).mockResolvedValue({ id: LEGACY_ID, ok: true, repo: 'demo', path: 'old.md' });
    clickOn(onKnowledgeLinkClick, `<a href="knowledge:${LEGACY_ID}">x</a>`, 'a');
    await flush();
    expect(resolveKnowledgeForOpen).toHaveBeenCalledWith(LEGACY_ID);
    expect(opened).toEqual([{ repo: 'demo', path: 'old.md' }]);
  });

  it('leaves every other link alone', async () => {
    const web = clickOn(onKnowledgeLinkClick, '<a href="https://example.com">x</a>', 'a');
    const note = clickOn(onKnowledgeLinkClick, `<a href="note:${ID}">x</a>`, 'a');
    const plain = clickOn(onKnowledgeLinkClick, '<span>no link</span>', 'span');
    for (const event of [web, note, plain]) expect(event.preventDefault).not.toHaveBeenCalled();
    await flush();
    expect(resolveKnowledgeForOpen).not.toHaveBeenCalled();
    expect(opened).toEqual([]);
  });

  it('reports a malformed knowledge: link without calling Rust', async () => {
    const event = clickOn(onKnowledgeLinkClick, '<a href="knowledge:zzz">x</a>', 'a');
    expect(event.preventDefault).toHaveBeenCalledOnce();
    await flush();
    expect(resolveKnowledgeForOpen).not.toHaveBeenCalled();
    expect(vi.mocked(showActionError).mock.calls[0][0].message).toMatch(/Cannot open knowledge document/);
    expect(opened).toEqual([]);
  });

  it('reports the Rust error when the id is unknown', async () => {
    vi.mocked(resolveKnowledgeForOpen).mockRejectedValue(new Error('Unknown document id'));
    clickOn(onKnowledgeLinkClick, `<a href="knowledge:${ID}">x</a>`, 'a');
    await flush();
    expect(vi.mocked(showActionError).mock.calls[0][0].message).toBe(
      'Cannot open knowledge document: Unknown document id',
    );
    expect(opened).toEqual([]);
  });

  it('does not open anything when Rust returns no repo or path', async () => {
    vi.mocked(resolveKnowledgeForOpen).mockResolvedValue({ id: ID, ok: true });
    clickOn(onKnowledgeLinkClick, `<a href="knowledge:${ID}">x</a>`, 'a');
    await flush();
    expect(opened).toEqual([]);
    expect(showActionError).toHaveBeenCalledOnce();
  });
});

describe('onChatLinkClick', () => {
  it('routes knowledge: links to the knowledge flow and never to notes', async () => {
    vi.mocked(resolveKnowledgeForOpen).mockResolvedValue({ id: ID, ok: true, repo: 'demo', path: 'a.md' });
    clickOn(onChatLinkClick, `<a href="knowledge:${ID}">x</a>`, 'a');
    await flush();
    expect(resolveKnowledgeForOpen).toHaveBeenCalledWith(ID);
    expect(resolveNoteForOpen).not.toHaveBeenCalled();
  });

  it('keeps routing note: links to the notes flow', async () => {
    vi.mocked(resolveNoteForOpen).mockRejectedValue(new Error('Entry not found'));
    clickOn(onChatLinkClick, `<a href="note:${ID}">x</a>`, 'a');
    await flush();
    expect(resolveNoteForOpen).toHaveBeenCalledWith(ID);
    expect(resolveKnowledgeForOpen).not.toHaveBeenCalled();
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
