// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';

const readGet = vi.fn(async () => ({ content: '# hi' }));
const writePost = vi.fn(async () => ({ ok: true }));

vi.mock('../../frontend/src/host/api/transport.ts', () => ({
  readGet: (...args) => readGet(...args),
  writePost: (...args) => writePost(...args),
}));

import { copyFilePopupPath, openFilePopup, saveFilePopup } from '../../frontend/src/file-popup/commands/popup.ts';
import { emptyFilePopupView, viewStore } from '../../frontend/src/file-popup/state/store.ts';

describe('openFilePopup / saveFilePopup', () => {
  afterEach(() => {
    viewStore.set(emptyFilePopupView());
    readGet.mockClear();
    writePost.mockClear();
  });

  it('loads with path only and stores identityKey', async () => {
    await openFilePopup({
      path: '/tmp/doc.md',
      title: 'Doc',
      identityKey: 'knowledge:demo/doc.md',
    });
    expect(readGet).toHaveBeenCalledTimes(1);
    const url = String(readGet.mock.calls[0][0]);
    expect(url.startsWith('/api/file?path=')).toBe(true);
    expect(url).toContain(encodeURIComponent('/tmp/doc.md'));
    expect(url).not.toContain('layer=');
    const view = viewStore.getSnapshot();
    expect(view.path).toBe('/tmp/doc.md');
    expect(view.title).toBe('Doc');
    expect(view.identityKey).toBe('knowledge:demo/doc.md');
    expect(view.content).toBe('# hi');
    expect(view).not.toHaveProperty('layer');
  });

  it('saves path and content without layer', async () => {
    viewStore.set({
      ...emptyFilePopupView(),
      open: true,
      path: '/tmp/doc.md',
      content: '# old',
    });
    await saveFilePopup();
    expect(writePost).toHaveBeenCalledWith('/api/file', {
      path: '/tmp/doc.md',
      content: '# old',
    });
  });

  it('copyFilePopupPath writes the stored absolute path', async () => {
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    viewStore.set({
      ...emptyFilePopupView(),
      open: true,
      path: '/Users/me/knowledge/demo/doc.md',
    });
    await expect(copyFilePopupPath()).resolves.toBe(true);
    expect(writeText).toHaveBeenCalledWith('/Users/me/knowledge/demo/doc.md');
  });

  it('copyFilePopupPath no-ops when path is empty', async () => {
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    await expect(copyFilePopupPath()).resolves.toBe(false);
    expect(writeText).not.toHaveBeenCalled();
  });
});
