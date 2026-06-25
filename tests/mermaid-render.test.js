// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { initMermaid, renderMermaidBlocks } from '../frontend/js/mermaid-render.js';

describe('initMermaid', () => {
  afterEach(() => {
    delete global.mermaid;
  });

  it('initializes mermaid once with strict security', () => {
    const initialize = vi.fn();
    global.mermaid = { initialize };

    initMermaid();
    initMermaid();

    expect(initialize).toHaveBeenCalledTimes(1);
    expect(initialize).toHaveBeenCalledWith({
      startOnLoad: false,
      securityLevel: 'strict',
    });
  });

  it('no-ops when mermaid global is missing', () => {
    expect(() => initMermaid()).not.toThrow();
  });
});

describe('renderMermaidBlocks', () => {
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

  it('no-ops on empty container', async () => {
    await expect(renderMermaidBlocks(null)).resolves.toBeUndefined();
  });

  it('no-ops when no mermaid blocks exist', async () => {
    const container = document.createElement('div');
    container.innerHTML = '<p>hello</p>';
    await renderMermaidBlocks(container);
    expect(container.querySelector('.mermaid-diagram')).toBeNull();
  });

  it('renders valid mermaid block to svg wrapper', async () => {
    const container = document.createElement('div');
    container.innerHTML = '<pre><code class="language-mermaid">graph TD; A-->B;</code></pre>';

    await renderMermaidBlocks(container);

    const diagram = container.querySelector('.mermaid-diagram');
    expect(diagram).not.toBeNull();
    expect(diagram.querySelector('svg')).not.toBeNull();
    expect(container.querySelector('pre')).toBeNull();
  });

  it('shows error fallback for invalid syntax', async () => {
    global.mermaid.render = vi.fn(async () => {
      throw new Error('Parse error');
    });

    const container = document.createElement('div');
    container.innerHTML = '<pre><code class="language-mermaid">not valid</code></pre>';

    await renderMermaidBlocks(container);

    const diagram = container.querySelector('.mermaid-diagram');
    expect(diagram.querySelector('.mermaid-error')).not.toBeNull();
    expect(diagram.querySelector('pre code.language-mermaid')).not.toBeNull();
  });

  it('renders multiple blocks with unique render ids', async () => {
    const container = document.createElement('div');
    container.innerHTML = [
      '<pre><code class="language-mermaid">graph TD; A-->B;</code></pre>',
      '<pre><code class="language-mermaid">graph LR; C-->D;</code></pre>',
    ].join('');

    await renderMermaidBlocks(container);

    expect(global.mermaid.render).toHaveBeenCalledTimes(2);
    const ids = global.mermaid.render.mock.calls.map(call => call[0]);
    expect(new Set(ids).size).toBe(2);
    expect(container.querySelectorAll('.mermaid-diagram').length).toBe(2);
  });
});
