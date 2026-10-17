// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { marked } from 'marked';

vi.mock('../../frontend/src/host/api/doc-open.ts', () => ({
  resolveDocForOpen: vi.fn(),
}));
vi.mock('../../frontend/src/notes/commands/reload-index.ts', () => ({
  refreshNotesIndex: vi.fn(),
}));
vi.mock('../../frontend/src/home/commands/hub.ts', () => ({
  showActionError: vi.fn(),
}));

import { resolveDocForOpen } from '../../frontend/src/host/api/doc-open.ts';
import { refreshNotesIndex } from '../../frontend/src/notes/commands/reload-index.ts';
import { showActionError } from '../../frontend/src/home/commands/hub.ts';
import { state } from '../../frontend/src/host/state.ts';
import {
  docIdFromHref,
  onDocLinkClick,
} from '../../frontend/src/home/commands/open-doc-link.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const ID = 'abcdefabcdefabcdefabcdefabcdefab';

function clickOn(html, selector) {
  const root = document.createElement('div');
  root.innerHTML = html;
  const event = { target: root.querySelector(selector), preventDefault: vi.fn() };
  onDocLinkClick(event);
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
  vi.mocked(resolveDocForOpen).mockReset();
  vi.mocked(refreshNotesIndex).mockReset();
  vi.mocked(showActionError).mockReset();
  state.index.data = { [ID]: { project_id: 'ai', id: ID } };
});

afterEach(() => {
  document.removeEventListener('cta:open-entry', onOpen);
  state.index.data = null;
});

describe('docIdFromHref', () => {
  it('accepts spark-doc:<id> only; the old doc:, note:, and knowledge: schemes are gone', () => {
    expect(docIdFromHref(`spark-doc:${ID}`)).toBe(ID);
    expect(docIdFromHref(`  spark-doc:${ID} `)).toBe(ID);
    expect(docIdFromHref(`spark-doc:default-${ID}`)).toBe(`default-${ID}`);
    expect(docIdFromHref(`spark-doc:k3x9ab-${ID}`)).toBe(`k3x9ab-${ID}`);
    for (const bad of [
      `doc:${ID}`,
      'spark-doc:bad',
      `note:${ID}`,
      `knowledge:${ID}`,
      `spark-doc:${ID}x`,
      `spark-doc:Bad-${ID}`,
      `spark-doc:-${ID}`,
      `https://x/${ID}`,
      `Note:${ID}`,
      '',
      null,
      undefined,
    ]) {
      expect(docIdFromHref(bad)).toBeNull();
    }
  });
});

describe('onDocLinkClick', () => {
  it('intercepts a spark-doc: link, resolves it in Rust, then opens the entry', async () => {
    vi.mocked(resolveDocForOpen).mockResolvedValue({ id: ID, ok: true, project_id: 'ai' });
    const event = clickOn(`<p><a href="spark-doc:${ID}"><b>Default：Title</b></a></p>`, 'b');
    expect(event.preventDefault).toHaveBeenCalledOnce();
    await flush();
    expect(resolveDocForOpen).toHaveBeenCalledWith(ID);
    expect(opened).toEqual([{ id: ID }]);
    expect(showActionError).not.toHaveBeenCalled();
  });

  it('leaves every other link alone', async () => {
    const event = clickOn('<a href="https://example.com">x</a>', 'a');
    expect(event.preventDefault).not.toHaveBeenCalled();
    for (const old of ['doc', 'note', 'knowledge']) {
      expect(clickOn(`<a href="${old}:${ID}">x</a>`, 'a').preventDefault).not.toHaveBeenCalled();
    }
    const plain = clickOn('<span>no link</span>', 'span');
    expect(plain.preventDefault).not.toHaveBeenCalled();
    await flush();
    expect(resolveDocForOpen).not.toHaveBeenCalled();
    expect(opened).toEqual([]);
  });

  it('reports a malformed spark-doc: link without calling Rust', async () => {
    const event = clickOn('<a href="spark-doc:bad">x</a>', 'a');
    expect(event.preventDefault).toHaveBeenCalledOnce();
    await flush();
    expect(resolveDocForOpen).not.toHaveBeenCalled();
    expect(showActionError).toHaveBeenCalledWith({ message: expect.stringContaining('Cannot open document') });
    expect(opened).toEqual([]);
  });

  it('reports the Rust error when the id is unknown', async () => {
    vi.mocked(resolveDocForOpen).mockRejectedValue(new Error('Entry not found'));
    clickOn(`<a href="spark-doc:${ID}">x</a>`, 'a');
    await flush();
    const err = vi.mocked(showActionError).mock.calls[0][0];
    expect(err.message).toBe('Cannot open document: Entry not found');
    expect(opened).toEqual([]);
  });

  it('refreshes a stale list once, then opens', async () => {
    state.index.data = {};
    vi.mocked(resolveDocForOpen).mockResolvedValue({ id: ID, ok: true, project_id: 'ai' });
    vi.mocked(refreshNotesIndex).mockImplementation(async () => {
      state.index.data = { [ID]: { project_id: 'ai', id: ID } };
      return true;
    });
    clickOn(`<a href="spark-doc:${ID}">x</a>`, 'a');
    await flush();
    expect(refreshNotesIndex).toHaveBeenCalledOnce();
    expect(opened).toEqual([{ id: ID }]);
  });

  it('reports instead of staying silent when the list still lacks the document', async () => {
    state.index.data = {};
    vi.mocked(resolveDocForOpen).mockResolvedValue({ id: ID, ok: true, project_id: 'ai' });
    vi.mocked(refreshNotesIndex).mockResolvedValue(true);
    clickOn(`<a href="spark-doc:${ID}">x</a>`, 'a');
    await flush();
    expect(opened).toEqual([]);
    expect(vi.mocked(showActionError).mock.calls[0][0].message).toMatch(/Cannot open document/);
  });
});

describe('wiring', () => {
  it('marked keeps the spark-doc: href intact', () => {
    const html = marked.parse(`[Title](spark-doc:${ID})`);
    expect(html).toContain(`href="spark-doc:${ID}"`);
  });

  it('assistant markdown in the home page uses the click handler', () => {
    const page = readFileSync(join(repoRoot, 'frontend/src/home/page.tsx'), 'utf8');
    expect(page).toMatch(/onClick=\{onChatLinkClick\}/);
  });
});
