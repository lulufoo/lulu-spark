// @vitest-environment jsdom
import { act } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchBoardPatStatus: async () => false,
  setBoardPat: async () => {},
  deleteBoardPat: async () => {},
}));

const { removeBoardPat, saveBoardPat } = await import(
  '../../frontend/src/app-shell/commands/settings/board.ts'
);
const { mountBoardSketches } = await import('../../frontend/src/board/commands/sketch-links.ts');

const PAGE = 'https://luluboard.app/view#b:b_0123abcd';

function deferred() {
  let resolve;
  const promise = new Promise((done) => { resolve = done; });
  return { promise, resolve };
}

beforeEach(() => {
  document.body.innerHTML = `<div id="doc"><a href="${PAGE}">Sketch</a></div>`;
});

describe('board token changes', () => {
  it('clears a shown preview when the token is removed', async () => {
    const doc = document.getElementById('doc');
    let reads = 0;
    await act(async () => {
      await mountBoardSketches(doc, () => {
        reads += 1;
        if (reads === 1) {
          return Promise.resolve({ status: 'sketch', content_b64: btoa('<p>Old</p>') });
        }
        return Promise.resolve({ status: 'unconfigured' });
      });
    });
    expect(doc.querySelector('iframe')).not.toBeNull();
    await act(async () => { await removeBoardPat(); });
    expect(doc.querySelector('iframe')).toBeNull();
    expect(reads).toBe(2);
  });

  it('drops a read that is still running when the token is removed', async () => {
    const doc = document.getElementById('doc');
    const first = deferred();
    let reads = 0;
    const pending = mountBoardSketches(doc, () => {
      reads += 1;
      if (reads === 1) return first.promise;
      return Promise.resolve({ status: 'unconfigured' });
    });
    await act(async () => {
      await removeBoardPat();
      first.resolve({ status: 'sketch', content_b64: btoa('<p>Old</p>') });
      await pending;
      await Promise.resolve();
    });
    expect(doc.querySelector('iframe')).toBeNull();
    expect(reads).toBe(2);
  });

  it('reads again after a new token is saved', async () => {
    const doc = document.getElementById('doc');
    let reads = 0;
    await act(async () => {
      await mountBoardSketches(doc, () => {
        reads += 1;
        return Promise.resolve({
          status: 'sketch',
          content_b64: btoa(reads === 1 ? '<p>Old</p>' : '<p>New</p>'),
        });
      });
    });
    await act(async () => { await saveBoardPat('lbp_example'); });
    expect(reads).toBe(2);
    const posted = [];
    const frame = doc.querySelector('iframe');
    frame.contentWindow.postMessage = (data) => posted.push(data);
    await act(async () => {
      window.dispatchEvent(new MessageEvent('message', {
        data: { type: 'lulu-sandbox:ready' },
        source: frame.contentWindow,
      }));
    });
    expect(posted[0].html).toBe('<p>New</p>');
  });
});
