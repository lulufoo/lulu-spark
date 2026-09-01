// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { FilePopup } from '../../frontend/src/file-popup/ui/popup.tsx';
import { emptyFilePopupView, patchView, viewStore } from '../../frontend/src/file-popup/state/store.ts';

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchNotesAssetAsBlobUrl: vi.fn(async () => 'blob:file-popup-test'),
  fetchDocHighlights: vi.fn(async () => ({ highlights: [] })),
}));

describe('FilePopup chrome', () => {
  let container;
  let root;

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
    viewStore.set(emptyFilePopupView());
  });

  function renderOpen() {
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    patchView({
      open: true,
      path: '/kb/notes/raw/inbox/a.md',
      title: 'Sample note',
      layer: 'raw',
      content: '# Hello',
      editing: false,
      loading: false,
      saving: false,
      error: '',
    });
    act(() => {
      root.render(createElement(FilePopup));
    });
  }

  it('hides Edit when the path is not a notes file', () => {
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    patchView({
      open: true,
      path: '/Users/me/knowledge/demo/doc.md',
      title: 'Knowledge doc',
      layer: 'raw',
      content: '# KB',
      editing: false,
      loading: false,
      saving: false,
      error: '',
    });
    act(() => {
      root.render(createElement(FilePopup));
    });
    const labels = [...container.querySelectorAll('button')].map((b) => b.textContent.trim());
    expect(labels).not.toContain('Edit');
    expect(labels).toContain('Close');
  });

  it('uses Notes viewer chrome classes instead of a global header', () => {
    renderOpen();
    const dialog = container.querySelector('#file-popup');
    expect(dialog).not.toBeNull();
    expect(dialog.querySelector('header')).toBeNull();
    expect(dialog.querySelector('.viewer-header')).not.toBeNull();
    expect(dialog.querySelector('.viewer-panel-title')?.textContent).toBe('Sample note');
    const body = dialog.querySelector('#file-popup-body');
    expect(body).not.toBeNull();
    expect(body.classList.contains('viewer-body')).toBe(true);
    expect(dialog.querySelector('#file-popup-edit-area')?.classList.contains('viewer-edit-area')).toBe(
      true,
    );
  });
});
