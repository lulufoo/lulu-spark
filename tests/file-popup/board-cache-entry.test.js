// @vitest-environment jsdom
import { act } from 'react';
import { marked } from 'marked';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { paintFilePopupDoc } from '../../frontend/src/file-popup/ui/paint.ts';
import { PAGE, cleanPages, deferred, flush, ready, source } from '../board/sketch-fixtures.js';

const board = vi.hoisted(() => ({ cached: vi.fn(), remote: vi.fn(), listen: vi.fn() }));

vi.mock('../../frontend/src/host/api/board-sketch.ts', () => ({
  fetchCachedBoardSketch: board.cached,
  fetchBoardSketch: board.remote,
  listenBoardCacheInvalidated: board.listen,
}));
vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchDocHighlights: vi.fn(async () => ({ highlights: [] })),
}));

beforeEach(() => {
  cleanPages();
  vi.clearAllMocks();
  vi.stubGlobal('marked', marked);
  board.listen.mockResolvedValue(() => {});
  document.body.innerHTML = '<div id="file-popup-body" class="viewer-body"></div><textarea id="file-popup-edit-area"></textarea>';
});

afterEach(() => {
  cleanPages();
  vi.unstubAllGlobals();
});

describe('File popup Board cache entry', () => {
  it('shows cached content through the default Host paths and ignores late remote content after entering edit mode', async () => {
    const remote = deferred();
    const content = `[Original Board link](${PAGE})\n\n[Web](https://example.com)`;
    let remoteFinished = false;
    board.cached.mockResolvedValue(source({ diagram_generation: 0 }));
    board.remote.mockImplementation(() => remote.promise.then((reply) => {
      remoteFinished = true;
      return reply;
    }));

    await act(async () => { await paintFilePopupDoc({ editing: false, content }); });
    await flush();
    const body = document.getElementById('file-popup-body');
    const frame = body.querySelector('iframe');
    expect(board.cached).toHaveBeenCalledExactlyOnceWith(PAGE);
    expect(board.remote).toHaveBeenCalledExactlyOnceWith(PAGE);
    expect(board.listen).toHaveBeenCalledTimes(1);
    const posted = await ready(frame);
    expect(posted[0].html).toBe('<p>Cached</p>');
    const box = body.querySelector('.board-sketch-frame-box');
    Object.defineProperty(box, 'clientWidth', { configurable: true, value: 480 });
    await act(async () => {
      window.dispatchEvent(new MessageEvent('message', {
        data: { type: 'lulu-sandbox:size', width: 320, height: 200 }, source: frame.contentWindow,
      }));
    });
    expect(frame.classList.contains('is-ready')).toBe(true);
    expect(body.querySelector('[aria-busy="true"]')).toBeNull();
    expect(remoteFinished).toBe(false);
    expect(body.querySelector('a[href="https://example.com"]').target).toBe('_blank');

    await act(async () => {
      await paintFilePopupDoc({ editing: true, content });
      remote.resolve(source({ version: 2, html: '<p>Late remote response</p>', diagram_generation: 1 }));
    });
    await flush();
    expect(body.querySelector('iframe')).toBeNull();
    expect(body.querySelector('a').hidden).toBe(false);
    expect(document.getElementById('file-popup-edit-area').value).toBe(content);
    expect(body.style.display).toBe('none');
    expect(board.cached).toHaveBeenCalledTimes(1);
    expect(board.remote).toHaveBeenCalledTimes(1);
  });
});
