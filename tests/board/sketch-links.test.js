// @vitest-environment jsdom
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import {
  mountBoardSketches,
  releaseBoardSketches,
} from '../../frontend/src/board/commands/sketch-links.ts';

const SKETCH = 'https://luluboard.app/view#b:b_0123abcd';
const OTHER = 'https://luluboard.app/view#b:b_89abcdef';
const PLAIN = 'https://example.com/notes';

function b64(value) {
  return btoa(value);
}

function page(html) {
  const host = document.createElement('div');
  host.innerHTML = html;
  document.body.appendChild(host);
  return host;
}

function deferred() {
  let resolve;
  const promise = new Promise((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

async function ready(frame) {
  const posted = [];
  frame.contentWindow.postMessage = (data) => posted.push(data);
  await act(async () => {
    window.dispatchEvent(new MessageEvent('message', {
      data: { type: 'lulu-sandbox:ready' },
      source: frame.contentWindow,
    }));
  });
  return posted;
}

beforeEach(() => {
  document.body.innerHTML = '';
  document.documentElement.dataset.theme = 'light';
  delete window.__TAURI__;
});

describe('markdown board sketches', () => {
  it('previews only a successful sketch and keeps every link', async () => {
    const source = `See ${SKETCH} and ${OTHER} and ${PLAIN}`;
    const host = page(
      `<p>${source}</p><a href="${SKETCH}">Sketch</a><a href="${OTHER}">Boxes</a><a href="${PLAIN}">Web</a>`,
    );
    const reads = [];
    await act(async () => {
      await mountBoardSketches(host, async (url) => {
        reads.push(url);
        if (url === SKETCH) return { status: 'sketch', content_b64: b64('<p>Hello</p>') };
        if (url === OTHER) return { status: 'not_sketch' };
        throw new Error('network');
      });
    });
    expect(reads).toEqual([SKETCH, OTHER]);
    expect(host.querySelectorAll('iframe')).toHaveLength(1);
    expect(host.querySelector('a[href="' + PLAIN + '"]')).not.toBeNull();
    expect(host.querySelector('a[href="' + OTHER + '"]')).not.toBeNull();
    const link = host.querySelector('a[href="' + SKETCH + '"]');
    expect(link.textContent).toBe('Sketch');
    expect(link.nextElementSibling.className).toBe('board-sketch-slot');
    const posted = await ready(host.querySelector('iframe'));
    expect(posted[0]).toEqual({ type: 'lulu-sandbox:render', html: '<p>Hello</p>', appearance: 'light' });
    expect(JSON.stringify(posted)).not.toContain('lbp_');
    expect(source).toBe(`See ${SKETCH} and ${OTHER} and ${PLAIN}`);
  });

  it('follows the document theme without a second read', async () => {
    document.documentElement.dataset.theme = 'dark';
    const host = page(`<a href="${SKETCH}">Sketch</a>`);
    let reads = 0;
    await act(async () => {
      await mountBoardSketches(host, async () => {
        reads += 1;
        return { status: 'sketch', content_b64: b64('<p>Hello</p>') };
      });
    });
    const posted = await ready(host.querySelector('iframe'));
    await act(async () => {
      document.documentElement.dataset.theme = 'light';
      await Promise.resolve();
    });
    expect(posted.map((item) => item.type)).toEqual(['lulu-sandbox:render', 'lulu-sandbox:appearance']);
    expect(posted[0].appearance).toBe('dark');
    expect(posted[1].appearance).toBe('light');
    expect(reads).toBe(1);
  });

  it('drops a sketch that belongs to the previous document', async () => {
    const host = page(`<a href="${SKETCH}">First</a>`);
    const first = deferred();
    const pending = mountBoardSketches(host, () => first.promise);
    releaseBoardSketches(host);
    host.innerHTML = `<a href="${OTHER}">Second</a>`;
    await act(async () => {
      const next = mountBoardSketches(host, async () => ({
        status: 'sketch',
        content_b64: b64('<p>Second</p>'),
      }));
      first.resolve({ status: 'sketch', content_b64: b64('<p>First</p>') });
      await pending;
      await next;
    });
    expect(host.querySelectorAll('iframe')).toHaveLength(1);
    const posted = await ready(host.querySelector('iframe'));
    expect(posted[0].html).toBe('<p>Second</p>');
    releaseBoardSketches(host);
    expect(host.querySelector('iframe')).toBeNull();
  });

  it('keeps the link when the sketch payload cannot be decoded', async () => {
    const host = page(`<a href="${SKETCH}">Sketch</a>`);
    await act(async () => {
      await mountBoardSketches(host, async () => ({ status: 'sketch', content_b64: '%%%' }));
    });
    expect(host.querySelector('iframe')).toBeNull();
    expect(host.querySelector('a').textContent).toBe('Sketch');
  });
});
