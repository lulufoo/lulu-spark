// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setGithubUserUrl } from '../../frontend/src/host/constants.ts';
import { state } from '../../frontend/src/host/state.ts';
import { paintFilePopupDoc } from '../../frontend/src/file-popup/ui/paint.ts';

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchNotesAssetAsBlobUrl: vi.fn(async () => 'blob:file-popup-test'),
  fetchDocHighlights: vi.fn(async () => ({ highlights: [] })),
}));

describe('paintFilePopupDoc', () => {
  beforeEach(() => {
    document.body.innerHTML =
      '<div id="file-popup-body" class="viewer-body"></div><textarea id="file-popup-edit-area"></textarea>';
    state.ui.workbenchRoot = '/Users/x/lulu-workbench-knowledge';
    setGithubUserUrl('https://github.com/lulu');
    global.marked = {
      parse(raw) {
        if (String(raw).includes('```mermaid')) {
          return '<pre><code class="language-mermaid">graph TD; A-->B;</code></pre>';
        }
        if (String(raw).includes('http://example.com')) {
          return '<p><a href="http://example.com">ex</a></p>';
        }
        if (String(raw).includes('sister.md')) {
          return '<p><a href="./sister.md">rel</a></p>';
        }
        if (String(raw).includes('diagram.png')) {
          return '<p><img src="diagram.png" alt=""></p>';
        }
        return `<h1>${String(raw).replace(/^#\s*/, '')}</h1>`;
      },
    };
    global.mermaid = {
      initialize: vi.fn(),
      render: vi.fn(async (_id, src) => ({
        svg: `<svg data-src="${src}"></svg>`,
        bindFunctions: vi.fn(),
      })),
    };
  });

  afterEach(() => {
    document.body.innerHTML = '';
    delete global.marked;
    delete global.mermaid;
    setGithubUserUrl('');
    state.ui.workbenchRoot = '';
  });

  it('renders mermaid fences after markdown', async () => {
    await paintFilePopupDoc({
      editing: false,
      content: '```mermaid\ngraph TD; A-->B;\n```',
      path: '/kb/notes/raw/inbox/a.md',
      layer: 'raw',
    });
    const diagram = document.querySelector('#file-popup-body .mermaid-diagram');
    expect(diagram).not.toBeNull();
    expect(diagram.querySelector('svg')).not.toBeNull();
  });

  it('opens http links in a new tab', async () => {
    await paintFilePopupDoc({
      editing: false,
      content: 'http://example.com',
      path: '/kb/notes/raw/inbox/a.md',
      layer: 'raw',
    });
    const a = document.querySelector('#file-popup-body a');
    expect(a.getAttribute('target')).toBe('_blank');
    expect(a.getAttribute('rel')).toContain('noopener');
  });

  it('rewrites relative note links to GitHub', async () => {
    await paintFilePopupDoc({
      editing: false,
      content: 'sister.md',
      path: '/kb/notes/raw/inbox/a.md',
      layer: 'raw',
    });
    const a = document.querySelector('#file-popup-body a');
    expect(a.getAttribute('href')).toBe(
      'https://github.com/lulu/lulu-workbench-knowledge/blob/main/notes/raw/inbox/sister.md',
    );
  });

  it('loads relative images through the notes asset API', async () => {
    const api = await import('../../frontend/src/host/api.ts');
    await paintFilePopupDoc({
      editing: false,
      content: 'diagram.png',
      path: '/kb/notes/raw/inbox/a.md',
      layer: 'raw',
    });
    expect(api.fetchNotesAssetAsBlobUrl).toHaveBeenCalledWith('raw', 'inbox/a.md', 'diagram.png');
    expect(document.querySelector('#file-popup-body img').getAttribute('src')).toBe(
      'blob:file-popup-test',
    );
  });

  it('skips markdown paint while editing', async () => {
    await paintFilePopupDoc({
      editing: true,
      content: '# Hello',
      path: '/kb/notes/raw/inbox/a.md',
      layer: 'raw',
    });
    expect(document.getElementById('file-popup-body').innerHTML).toBe('');
    expect(document.getElementById('file-popup-edit-area').style.display).toBe('');
  });
});
