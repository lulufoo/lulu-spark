// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { FilePopup } from '../../frontend/src/file-popup/ui/popup.tsx';
import { emptyFilePopupView, patchView, viewStore } from '../../frontend/src/file-popup/state/store.ts';

vi.mock('../../frontend/src/host/api.ts', () => ({
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

  function renderOpen(partial = {}) {
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    patchView({
      open: true,
      path: '/kb/notes/raw/inbox/a.md',
      title: 'Sample note',
      identityKey: 'notes:inbox/a.md',
      content: '# Hello',
      editing: false,
      loading: false,
      saving: false,
      error: '',
      ...partial,
    });
    act(() => {
      root.render(createElement(FilePopup));
    });
  }

  it('shows Edit whenever a path is open', () => {
    renderOpen({
      path: '/Users/me/knowledge/demo/doc.md',
      title: 'Knowledge doc',
      identityKey: 'knowledge:demo/doc.md',
      content: '# KB',
    });
    const labels = [...container.querySelectorAll('button')].map((b) => b.textContent.trim());
    expect(labels).toContain('Edit');
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
