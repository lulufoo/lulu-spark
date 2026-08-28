// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  normalizeTableSeparators,
  pasteTextFromClipboard,
  renderCommentMarkdown,
} from '../../frontend/src/shared/comment-markdown.ts';

describe('normalizeTableSeparators', () => {
  it('replaces em dash in GFM separator row', () => {
    const md = '| 层 | 名称 | 做什么 |\n|---|---|—|\n| a | b | c |';
    expect(normalizeTableSeparators(md)).toBe('| 层 | 名称 | 做什么 |\n|---|---|---|\n| a | b | c |');
  });

  it('leaves data rows unchanged', () => {
    const md = '| **L1** | 认知层 |';
    expect(normalizeTableSeparators(md)).toBe(md);
  });
});

describe('pasteTextFromClipboard', () => {
  beforeEach(() => {
    global.TurndownService = class {
      turndown(html) { return `TURNDOWN:${html.length}`; }
    };
  });

  it('prefers plain text when present', () => {
    const data = {
      getData: (type) => (type === 'text/plain' ? '| a | b |\n|---|---|' : '<table></table>'),
    };
    expect(pasteTextFromClipboard(data)).toBe('| a | b |\n|---|---|');
  });

  it('normalizes em-dash separator rows from plain clipboard', () => {
    const data = {
      getData: (type) => (
        type === 'text/plain'
          ? '| a | b |\n|---|---|—|—|\n| 1 | 2 |'
          : '<table></table>'
      ),
    };
    expect(pasteTextFromClipboard(data)).toBe('| a | b |\n|---|---|---|---|\n| 1 | 2 |');
  });

  it('uses Turndown when plain is empty', () => {
    const data = {
      getData: (type) => (type === 'text/plain' ? '' : type === 'text/html' ? '<p>hi</p>' : ''),
    };
    expect(pasteTextFromClipboard(data)).toBe('TURNDOWN:9');
  });

  it('returns plain when no html', () => {
    const data = { getData: (type) => (type === 'text/plain' ? 'hello' : '') };
    expect(pasteTextFromClipboard(data)).toBe('hello');
  });
});

describe('renderCommentMarkdown', () => {
  beforeEach(() => {
    global.marked = { parse: vi.fn((s) => `<parsed>${s}</parsed>`) };
  });

  it('normalizes separators before marked.parse', () => {
    const md = '| a | b | c |\n|---|---|—|\n| 1 | 2 | 3 |';
    renderCommentMarkdown(md);
    expect(marked.parse).toHaveBeenCalledWith('| a | b | c |\n|---|---|---|\n| 1 | 2 | 3 |');
  });
});
