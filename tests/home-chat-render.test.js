// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  renderHomeChatMarkdown,
  hydrateHomeChatMarkdown,
} from '../frontend/js/components/home-chat-render.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const source = readFileSync(
  join(fixtureRoot, 'frontend/js/components/home-chat-render.js'),
  'utf8',
);

describe('home-chat-render isolation', () => {
  it('is a standalone chat renderer and does not import the Notes editor', () => {
    expect(source).toMatch(/mermaid-render/);
    expect(source).not.toMatch(/viewer\.js|kb-viewer|comment-markdown/);
  });
});

describe('renderHomeChatMarkdown', () => {
  afterEach(() => {
    delete global.marked;
  });

  it('parses markdown through marked when present', () => {
    global.marked = { parse: vi.fn((md) => `<h2>${md}</h2>`) };
    expect(renderHomeChatMarkdown('## Hello')).toBe('<h2>## Hello</h2>');
    expect(global.marked.parse).toHaveBeenCalledWith('## Hello');
  });

  it('falls back to escaped pre when marked is missing', () => {
    delete global.marked;
    expect(renderHomeChatMarkdown('<em>x</em>')).toBe('<pre>&lt;em&gt;x&lt;/em&gt;</pre>');
  });

  it('treats null as empty', () => {
    global.marked = { parse: vi.fn((md) => `OK:${md}`) };
    expect(renderHomeChatMarkdown(null)).toBe('OK:');
  });
});

describe('hydrateHomeChatMarkdown', () => {
  beforeEach(() => {
    global.mermaid = {
      initialize: vi.fn(),
      render: vi.fn(async (_id, src) => ({
        svg: `<svg data-src="${src}"></svg>`,
        bindFunctions: vi.fn(),
      })),
    };
  });

  afterEach(() => {
    delete global.mermaid;
  });

  it('turns mermaid fences into svg via the shared mermaid helper', async () => {
    const container = document.createElement('div');
    container.innerHTML =
      '<pre><code class="language-mermaid">graph TD; A-->B;</code></pre>';

    await hydrateHomeChatMarkdown(container);

    expect(container.querySelector('.mermaid-diagram svg')).not.toBeNull();
    expect(container.querySelector('pre')).toBeNull();
  });

  it('no-ops on a missing container', async () => {
    await expect(hydrateHomeChatMarkdown(null)).resolves.toBeUndefined();
  });
});
