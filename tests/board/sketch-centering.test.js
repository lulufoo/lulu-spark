// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it } from 'vitest';
import { SketchPreview } from '../../frontend/src/board/ui/sketch-preview.tsx';

const PAGE = 'https://luluboard.app/view?utm=1#b:b_0123abcd';
const HTML = '<p>Hello</p>';

function render(props) {
  const host = document.createElement('div');
  document.body.appendChild(host);
  const root = createRoot(host);
  act(() => root.render(createElement(SketchPreview, props)));
  return { host, root };
}

beforeEach(() => { document.body.innerHTML = ''; });

describe('sketch preview centering', () => {
  function postSize(frame, width, height) {
    act(() => {
      window.dispatchEvent(new MessageEvent('message', {
        data: { type: 'lulu-sandbox:size', width, height },
        source: frame.contentWindow,
      }));
    });
  }

  function stubWidth(host, value) {
    const box = host.querySelector('.board-sketch-frame-box');
    Object.defineProperty(box, 'clientWidth', { configurable: true, get: () => value });
    return box;
  }

  it('centers a narrow sketch at its natural size with equal margins', () => {
    const { host, root } = render({ html: HTML, pageUrl: PAGE, appearance: 'light' });
    const frame = host.querySelector('iframe');
    frame.contentWindow.postMessage = () => {};
    stubWidth(host, 1460);
    postSize(frame, 1280, 720);
    expect(frame.style.transform).toBe('translateX(90px) scale(1)');
    expect(frame.style.width).toBe('1280px');
    expect(host.querySelector('.board-sketch-frame-box').style.height).toBe('720px');
    act(() => root.unmount());
  });

  it('scales a wide sketch to the container width with no offset', () => {
    const { host, root } = render({ html: HTML, pageUrl: PAGE, appearance: 'light' });
    const frame = host.querySelector('iframe');
    frame.contentWindow.postMessage = () => {};
    stubWidth(host, 1200);
    postSize(frame, 2400, 800);
    expect(frame.style.transform).toBe('translateX(0px) scale(0.5)');
    expect(host.querySelector('.board-sketch-frame-box').style.height).toBe('400px');
    act(() => root.unmount());
  });

  it('recomputes the centering on container resize without reloading the frame', () => {
    const observed = [];
    class FakeResizeObserver {
      constructor(callback) { observed.push(callback); }
      observe() {}
      disconnect() {}
    }
    globalThis.ResizeObserver = FakeResizeObserver;
    try {
      const { host, root } = render({ html: HTML, pageUrl: PAGE, appearance: 'light' });
      const frame = host.querySelector('iframe');
      frame.contentWindow.postMessage = () => {};
      stubWidth(host, 1000);
      postSize(frame, 800, 400);
      expect(frame.style.transform).toBe('translateX(100px) scale(1)');
      stubWidth(host, 1200);
      act(() => { observed.forEach((callback) => callback([])); });
      expect(frame.style.transform).toBe('translateX(200px) scale(1)');
      expect(host.querySelectorAll('iframe')).toHaveLength(1);
      expect(frame.getAttribute('src')).toBe('https://sandbox.luluboard.app/');
      act(() => root.unmount());
    } finally {
      delete globalThis.ResizeObserver;
    }
  });

  it('keeps sub-pixel offsets instead of rounding to the wrong side', () => {
    const { host, root } = render({ html: HTML, pageUrl: PAGE, appearance: 'light' });
    const frame = host.querySelector('iframe');
    frame.contentWindow.postMessage = () => {};
    stubWidth(host, 1461);
    postSize(frame, 1280, 720);
    expect(frame.style.transform).toBe('translateX(90.5px) scale(1)');
    act(() => root.unmount());
  });
});
