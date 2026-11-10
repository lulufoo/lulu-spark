// @vitest-environment jsdom
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { mountBoardSketches } from '../../frontend/src/board/commands/sketch-links.ts';
import { closeFilePopup } from '../../frontend/src/file-popup/commands/popup.ts';

const PAGE = 'https://luluboard.app/view#b:b_0123abcd';

beforeEach(() => {
  document.body.innerHTML = `<div id="file-popup-body"><a href="${PAGE}">Sketch</a></div>`;
});

describe('file popup close', () => {
  it('unmounts the sketch preview before the dialog goes away', async () => {
    const body = document.getElementById('file-popup-body');
    await act(async () => {
      await mountBoardSketches(body, async () => ({
        status: 'sketch',
        content_b64: btoa('<p>Hi</p>'),
      }));
    });
    const frame = body.querySelector('iframe');
    expect(frame).not.toBeNull();
    let heard = 0;
    frame.contentWindow.postMessage = () => { heard += 1; };
    window.dispatchEvent(new MessageEvent('message', {
      data: { type: 'lulu-sandbox:ready' },
      source: frame.contentWindow,
    }));
    expect(heard).toBe(1);
    closeFilePopup();
    expect(body.querySelector('iframe')).toBeNull();
    window.dispatchEvent(new MessageEvent('message', {
      data: { type: 'lulu-sandbox:ready' },
      source: frame.contentWindow,
    }));
    expect(heard).toBe(1);
  });
});
