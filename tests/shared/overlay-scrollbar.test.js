// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { attachOverlayScrollbar } from '../../frontend/src/shared/overlay-scrollbar.ts';

function makeScroller({ scrollHeight, clientHeight }) {
  const host = document.createElement('div');
  const scroller = document.createElement('div');
  Object.defineProperty(scroller, 'scrollHeight', { configurable: true, value: scrollHeight });
  Object.defineProperty(scroller, 'clientHeight', { configurable: true, value: clientHeight });
  Object.defineProperty(scroller, 'scrollTop', { configurable: true, value: 0, writable: true });
  scroller.getBoundingClientRect = () => ({ left: 0, top: 40, width: 248, height: clientHeight });
  host.getBoundingClientRect = () => ({ left: 0, top: 0, width: 248, height: 680 });
  host.appendChild(scroller);
  document.body.appendChild(host);
  return { host, scroller };
}

function trackOf(host) {
  return host.querySelector('.overlay-scrollbar-track');
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('overlay scrollbar', () => {
  it('stays hidden until the list scrolls, then reveals a thumb sized by the scroll fraction', () => {
    const { host, scroller } = makeScroller({ scrollHeight: 1200, clientHeight: 600 });
    const detach = attachOverlayScrollbar(scroller);
    const track = trackOf(host);
    expect(track.style.opacity).toBe(''); // CSS keeps the track hidden by default
    scroller.scrollTop = 0;
    scroller.dispatchEvent(new Event('scroll'));
    expect(track.style.display).toBe('block');
    expect(track.style.opacity).toBe('1');
    expect(track.style.top).toBe('40px');
    expect(track.style.height).toBe('600px');
    const thumb = track.querySelector('.overlay-scrollbar-thumb');
    // 600 of 1200 visible → half of the 596px track, min 28px.
    expect(thumb.style.height).toBe('298px');
    expect(thumb.style.top).toBe('2px');
    detach();
    expect(trackOf(host)).toBeNull();
  });

  it('follows the scroll position and fades out after scrolling stops', () => {
    const { host, scroller } = makeScroller({ scrollHeight: 1200, clientHeight: 600 });
    const detach = attachOverlayScrollbar(scroller);
    scroller.scrollTop = 300; // half the scrollable range
    scroller.dispatchEvent(new Event('scroll'));
    const thumb = trackOf(host).querySelector('.overlay-scrollbar-thumb');
    expect(thumb.style.top).toBe('151px'); // 2 + 149
    vi.advanceTimersByTime(700);
    expect(trackOf(host).style.opacity).toBe('0');
    detach();
  });

  it('keeps hiding when a scroll event lands on a list that does not overflow', () => {
    const { host, scroller } = makeScroller({ scrollHeight: 600, clientHeight: 600 });
    const detach = attachOverlayScrollbar(scroller);
    scroller.dispatchEvent(new Event('scroll'));
    expect(trackOf(host).style.display).toBe('none');
    expect(trackOf(host).style.opacity).toBe('');
    detach();
  });

  it('restores a static host on detach and keeps a positioned host as is', () => {
    const { host, scroller } = makeScroller({ scrollHeight: 1200, clientHeight: 600 });
    const detach = attachOverlayScrollbar(scroller);
    expect(host.style.position).toBe('relative');
    detach();
    expect(host.style.position).toBe('');

    host.style.position = 'absolute';
    const detach2 = attachOverlayScrollbar(scroller);
    expect(host.style.position).toBe('absolute');
    detach2();
    expect(host.style.position).toBe('absolute');
  });
});
