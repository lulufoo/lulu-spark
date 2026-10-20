// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { initMermaid, renderMermaidBlocks } from '../../frontend/src/shared/mermaid-render.ts';

describe('initMermaid', () => {
  afterEach(() => {
    delete global.pele;
  });

  it('no-ops when the renderer global is missing', () => {
    expect(() => initMermaid()).not.toThrow();
  });

  it('no-ops when the renderer global is present', () => {
    global.pele = { render: vi.fn() };
    expect(() => initMermaid()).not.toThrow();
  });
});

describe('renderMermaidBlocks', () => {
  beforeEach(() => {
    global.pele = {
      render: vi.fn((src) => ({
        svg: `<svg data-src="${src}"></svg>`,
      })),
      enableZoom: vi.fn(),
    };
  });

  afterEach(() => {
    delete global.pele;
    delete global.mermaid;
  });

  it('no-ops on empty container', async () => {
    await expect(renderMermaidBlocks(null)).resolves.toBeUndefined();
  });

  it('no-ops when the renderer global is missing', async () => {
    delete global.pele;
    const container = document.createElement('div');
    container.innerHTML = '<pre><code class="language-mermaid">graph TD; A-->B;</code></pre>';
    await renderMermaidBlocks(container);
    expect(container.querySelector('.mermaid-diagram')).toBeNull();
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
    global.pele.render = vi.fn(() => {
      throw new Error('Parse error');
    });

    const container = document.createElement('div');
    container.innerHTML = '<pre><code class="language-mermaid">not valid</code></pre>';

    await renderMermaidBlocks(container);

    const diagram = container.querySelector('.mermaid-diagram');
    expect(diagram.querySelector('.mermaid-error')).not.toBeNull();
    expect(diagram.querySelector('pre code.language-mermaid')).not.toBeNull();
    expect(diagram.querySelector('svg')).toBeNull();
  });

  it('does not fall back to a second parser when render throws', async () => {
    const mermaidRender = vi.fn(async () => ({ svg: '<svg data-fallback="1"></svg>' }));
    global.mermaid = { initialize: vi.fn(), render: mermaidRender };
    global.pele.render = vi.fn(() => {
      throw new Error('Parse error');
    });

    const container = document.createElement('div');
    container.innerHTML = '<pre><code class="language-mermaid">not valid</code></pre>';

    await renderMermaidBlocks(container);

    expect(mermaidRender).not.toHaveBeenCalled();
    expect(container.querySelector('.mermaid-error')).not.toBeNull();
    expect(container.querySelector('svg')).toBeNull();
  });

  it('renders multiple blocks', async () => {
    const container = document.createElement('div');
    container.innerHTML = [
      '<pre><code class="language-mermaid">graph TD; A-->B;</code></pre>',
      '<pre><code class="language-mermaid">graph LR; C-->D;</code></pre>',
    ].join('');

    await renderMermaidBlocks(container);

    expect(global.pele.render).toHaveBeenCalledTimes(2);
    expect(container.querySelectorAll('.mermaid-diagram').length).toBe(2);
  });
});
