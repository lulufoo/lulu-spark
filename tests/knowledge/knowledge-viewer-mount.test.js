// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const {
  fetchKbFileContent,
  fetchKbAnnotation,
  fetchKbStatus,
  fetchKbAssetAsBlobUrl,
} = vi.hoisted(() => ({
  fetchKbFileContent: vi.fn(),
  fetchKbAnnotation: vi.fn(),
  fetchKbStatus: vi.fn(),
  fetchKbAssetAsBlobUrl: vi.fn(),
}));

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchKbFileContent,
  fetchKbAnnotation,
  fetchKbStatus,
  fetchKbAssetAsBlobUrl,
  saveKbFile: vi.fn(),
  commitKbFile: vi.fn(),
  revertKbFile: vi.fn(),
}));

vi.mock('../../frontend/src/knowledge/ui/comments.tsx', () => ({
  renderKbComments: vi.fn(),
  initKbComments: vi.fn(),
  cleanupKbComments: vi.fn(),
  KbCommentsBar: () => null,
  KbCommentFloatNav: () => null,
}));

vi.mock('../../frontend/src/doc-editor/highlights.ts', () => ({
  applyCachedHighlights: vi.fn(),
  initDocHighlightOverlay: vi.fn(),
  cleanupDocHighlightOverlay: vi.fn(),
}));

vi.mock('../../frontend/src/knowledge/ui/links-bar.tsx', () => ({
  renderKbLinksBar: vi.fn(),
  KbLinksBar: () => null,
}));

vi.mock('../../frontend/src/shared/mermaid-render.ts', () => ({
  renderMermaidBlocks: vi.fn().mockResolvedValue(undefined),
}));

import { state } from '../../frontend/src/host/state.ts';
import { mountKbReader } from '../../frontend/src/knowledge/viewer.ts';

function deferred() {
  let resolve;
  const promise = new Promise((r) => { resolve = r; });
  return { promise, resolve };
}

describe('mountKbReader', () => {
  let container;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    document.body.style.overflow = '';
    state.viewer.isKb = false;
    state.viewer.kbRepo = null;
    state.viewer.kbPath = null;
    state.viewer.rawText = '';
    state.viewer.annotation = {};
    vi.clearAllMocks();
    fetchKbFileContent.mockResolvedValue({ content: '# Hello\n\nworld' });
    fetchKbAnnotation.mockResolvedValue({});
    fetchKbStatus.mockResolvedValue({ error: null, total: 0, ahead: 0 });
    globalThis.marked = { parse: (text) => `<p>${text}</p>` };
  });

  afterEach(() => {
    container.remove();
    delete globalThis.marked;
  });

  async function flushPromises() {
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
  }

  it('renders toolbar and body inside container', async () => {
    const { unmount } = await mountKbReader(container, {
      repo: 'owner/repo',
      path: 'docs/guide.md',
      url: 'https://github.com/owner/repo/blob/main/docs/guide.md',
    });

    expect(container.querySelector('.kb-reader-header')).toBeTruthy();
    expect(container.querySelector('.kb-reader-title')).toBeNull();
    expect(container.querySelector('.kb-reader-body')).toBeTruthy();
    expect(container.querySelector('.kb-btn-edit')).toBeTruthy();
    expect(container.querySelector('.kb-btn-reindex')).toBeNull();
    const openInChat = container.querySelector('.kb-btn-open-in-chat');
    expect(openInChat).toBeTruthy();
    expect(openInChat.getAttribute('aria-label')).toBe('Open in chat');
    expect(openInChat.getAttribute('title')).toBe('Open in chat');
    unmount();
  });

  it('sets isKb and shows content after fetch', async () => {
    const { unmount } = await mountKbReader(container, {
      repo: 'owner/repo',
      path: 'readme.md',
    });
    await flushPromises();

    expect(state.viewer.isKb).toBe(true);
    expect(state.viewer.kbRepo).toBe('owner/repo');
    expect(state.viewer.kbPath).toBe('readme.md');
    expect(container.querySelector('.kb-reader-body').innerHTML).toContain('Hello');
    unmount();
  });

  it('unmount clears container and resets isKb', async () => {
    const { unmount } = await mountKbReader(container, {
      repo: 'owner/repo',
      path: 'readme.md',
    });
    unmount();
    expect(container.innerHTML).toBe('');
    expect(state.viewer.isKb).toBe(false);
  });

  it('does not lock body overflow', async () => {
    const { unmount } = await mountKbReader(container, {
      repo: 'owner/repo',
      path: 'readme.md',
    });
    expect(document.body.style.overflow).toBe('');
    unmount();
  });

  it('shows error message when fetch fails', async () => {
    fetchKbFileContent.mockRejectedValue(new Error('network down'));
    const { unmount } = await mountKbReader(container, {
      repo: 'owner/repo',
      path: 'missing.md',
    });
    await flushPromises();
    expect(container.textContent).toContain('Could not load file');
    expect(container.textContent).toContain('network down');
    unmount();
  });

  it('ignores stale fetch after unmount', async () => {
    const pending = deferred();
    fetchKbFileContent.mockReturnValue(pending.promise);
    const first = await mountKbReader(container, {
      repo: 'owner/repo',
      path: 'a.md',
    });
    first.unmount();
    pending.resolve({ content: 'stale' });
    await Promise.resolve();
    expect(container.innerHTML).toBe('');
  });

  it('hides viewer-body in edit mode so the textarea fills the row', async () => {
    const { unmount } = await mountKbReader(container, {
      repo: 'owner/repo',
      path: 'a.md',
    });
    await flushPromises();
    container.querySelector('.kb-btn-edit').click();
    expect(container.querySelector('.viewer-body').style.display).toBe('none');
    expect(container.querySelector('.kb-reader-edit-area').style.display).not.toBe('none');
    container.querySelector('.kb-btn-cancel-edit').click();
    expect(container.querySelector('.viewer-body').style.display).toBe('');
    expect(container.querySelector('.kb-reader-edit-area').style.display).toBe('none');
    unmount();
  });

  it('exits edit mode before loading a new file', async () => {
    const { unmount } = await mountKbReader(container, {
      repo: 'owner/repo',
      path: 'a.md',
    });
    container.querySelector('.kb-btn-edit').click();
    expect(container.querySelector('.kb-reader-edit-area').style.display).not.toBe('none');

    const second = await mountKbReader(container, {
      repo: 'owner/repo',
      path: 'b.md',
    });
    await flushPromises();
    expect(container.querySelector('.kb-reader-edit-area').style.display).toBe('none');
    second.unmount();
    unmount();
  });

  it('cleans up listeners after repeated mount/unmount', async () => {
    const addSpy = vi.spyOn(HTMLElement.prototype, 'addEventListener');
    const removeSpy = vi.spyOn(HTMLElement.prototype, 'removeEventListener');

    for (let i = 0; i < 10; i += 1) {
      const { unmount } = await mountKbReader(container, {
        repo: 'owner/repo',
        path: `file-${i}.md`,
      });
      unmount();
    }

    expect(addSpy.mock.calls.length).toBeGreaterThan(0);
    expect(removeSpy.mock.calls.length).toBeGreaterThan(0);
    addSpy.mockRestore();
    removeSpy.mockRestore();
    expect(state.viewer.isKb).toBe(false);
    expect(document.body.style.overflow).toBe('');
  });
});
