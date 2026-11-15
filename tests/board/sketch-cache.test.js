// @vitest-environment jsdom
import { act } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mountBoardSketches } from '../../frontend/src/board/commands/sketch-links.ts';
import * as sketchLog from '../../frontend/src/board/commands/sketch-log.ts';
import { PAGE, cleanPages, deferred, flush, page, readers, ready, source } from './sketch-fixtures.js';

beforeEach(cleanPages);
afterEach(cleanPages);

describe('Board cache first preview', () => {
  it('uses its prewarmed iframe for cache while remote is suspended', async () => {
    const host = page(PAGE, 'Original  Board ↔ link');
    const remote = deferred();
    const order = [];
    const transport = readers({
      readCached: async () => { order.push('cache'); return source({ diagram_generation: 0 }); },
      readRemote: () => { order.push('remote'); return remote.promise; },
    });
    const listen = transport.listenInvalidated;
    transport.listenInvalidated = async (callback) => { order.push('listen'); return listen(callback); };
    const pending = mountBoardSketches(host, transport);
    const frame = host.querySelector('iframe');
    expect(frame).not.toBeNull();
    await flush();
    expect(order).toEqual(['listen', 'cache', 'remote']);
    expect(host.querySelector('iframe')).toBe(frame);
    const posted = await ready(frame);
    expect(posted[0].html).toBe('<p>Cached</p>');
    const box = host.querySelector('.board-sketch-frame-box');
    Object.defineProperty(box, 'clientWidth', { configurable: true, value: 480 });
    await act(async () => {
      window.dispatchEvent(new MessageEvent('message', {
        data: { type: 'lulu-sandbox:size', width: 320, height: 200 }, source: frame.contentWindow,
      }));
    });
    expect(frame.classList.contains('is-ready')).toBe(true);
    expect(host.querySelector('[aria-busy="true"]')).toBeNull();
    expect(host.querySelector('a').textContent).toBe('Original  Board ↔ link');
    expect(host.querySelector('a').getAttribute('href')).toBe(PAGE);
    await act(async () => { remote.resolve({ status: 'network' }); await pending; });
    expect(host.querySelector('iframe')).toBe(frame);
  });

  it('keeps equal-version content and title while accepting its fresher identity', async () => {
    const host = page();
    const remote = deferred();
    const transport = readers({ readCached: async () => source(), readRemote: () => remote.promise });
    const pending = mountBoardSketches(host, transport);
    await flush();
    const frame = host.querySelector('iframe');
    const posted = await ready(frame);
    await act(async () => {
      remote.resolve(source({ html: '<p>Changed body</p>', title: 'Changed title', diagram_generation: 3 }));
      await pending;
      transport.emit({ ...source(), diagram_generation: 2, reason: 'forbidden' });
    });
    expect(host.querySelector('iframe')).toBe(frame);
    expect(frame.title).toBe('Cached title');
    expect(posted.filter((message) => message.type === 'lulu-sandbox:render')).toHaveLength(1);
    expect(posted[0].html).toBe('<p>Cached</p>');
  });

  it.each(['<p>New content</p>', '<p>Cached</p>'])('replaces only its iframe once for a changed version (%s)', async (html) => {
    const host = page();
    const sibling = document.createElement('button');
    host.appendChild(sibling);
    const remote = deferred();
    const pending = mountBoardSketches(host, readers({
      readCached: async () => source(), readRemote: () => remote.promise,
    }));
    await flush();
    const first = host.querySelector('iframe');
    const firstPosted = await ready(first);
    const slot = host.querySelector('.board-sketch-slot');
    await act(async () => { remote.resolve(source({ html, version: 2, diagram_generation: 2 })); await pending; });
    const next = host.querySelector('iframe');
    expect(next).not.toBe(first);
    expect(host.querySelectorAll('iframe')).toHaveLength(1);
    expect(host.querySelector('.board-sketch-slot')).toBe(slot);
    expect(host.lastElementChild).toBe(sibling);
    expect(firstPosted).toHaveLength(1);
    const posted = await ready(next);
    expect(posted[0].html).toBe(html);
    expect(posted).toHaveLength(1);
  });

  it('does not wait for cache before painting remote and ignores the late cache', async () => {
    const host = page();
    const cache = deferred();
    const pending = mountBoardSketches(host, readers({
      readCached: () => cache.promise,
      readRemote: async () => source({ html: '<p>Remote</p>', version: 2, diagram_generation: 2 }),
    }));
    await flush();
    const frame = host.querySelector('iframe');
    const posted = await ready(frame);
    expect(posted[0].html).toBe('<p>Remote</p>');
    await act(async () => { cache.resolve(source({ status: 'not_sketch' })); await pending; });
    expect(host.querySelector('iframe')).toBe(frame);
    expect(host.querySelector('.board-sketch-note')).toBeNull();
  });

  it('returns to the link after remote failure while still accepting a hung cache when it arrives', async () => {
    const host = page();
    const cache = deferred();
    const pending = mountBoardSketches(host, readers({
      readCached: () => cache.promise, readRemote: async () => ({ status: 'network' }),
    }));
    await flush();
    expect(host.querySelector('iframe')).toBeNull();
    expect(host.querySelector('a').hidden).toBe(false);
    await act(async () => { cache.resolve(source()); await pending; });
    expect(host.querySelector('iframe')).not.toBeNull();
  });

  it.each(['network', 'unavailable', 'invalid_document', 'throw'])('silently retains a valid cache after %s', async (status) => {
    const host = page();
    const remote = deferred();
    const pending = mountBoardSketches(host, readers({ readCached: async () => source(), readRemote: () => remote.promise }));
    await flush();
    const frame = host.querySelector('iframe');
    await act(async () => {
      if (status === 'throw') remote.reject(new Error('offline'));
      else remote.resolve({ status });
      await pending;
    });
    expect(host.querySelector('iframe')).toBe(frame);
    expect(host.querySelector('.board-sketch-error')).toBeNull();
    expect(host.querySelector('.board-sketch-note')).toBeNull();
  });

  it.each(['not_sketch', 'invalid_source'])('can replace cached %s fallback with a remote Sketch', async (status) => {
    const host = page();
    const remote = deferred();
    const pending = mountBoardSketches(host, readers({
      readCached: async () => source({ status }), readRemote: () => remote.promise,
    }));
    await flush();
    expect(host.querySelector('iframe')).toBeNull();
    expect(host.querySelector('a').hidden).toBe(false);
    await act(async () => { remote.resolve(source({ version: 2, diagram_generation: 2 })); await pending; });
    expect(host.querySelector('iframe')).not.toBeNull();
    expect(host.querySelector('.board-sketch-note')).toBeNull();
  });

  it.each(['not_sketch', 'invalid_source'])('returns a cached Sketch to its original link for fresh %s', async (status) => {
    const host = page();
    const remote = deferred();
    const transport = readers({ readCached: async () => source(), readRemote: () => remote.promise });
    const pending = mountBoardSketches(host, transport);
    await flush();
    expect(host.querySelector('iframe')).not.toBeNull();
    await act(async () => { remote.resolve(source({ status, version: 2, diagram_generation: 2 })); await pending; });
    expect(host.querySelector('iframe')).toBeNull();
    expect(host.querySelector('a').hidden).toBe(false);
    expect(host.querySelector('a').textContent).toBe('Original Board link');
    expect(transport.stops).toBe(0);
  });

  it('logs cache and remote read time separately from sandbox time without source text', async () => {
    const spy = vi.spyOn(sketchLog, 'logSketch').mockImplementation(() => {});
    const host = page();
    await act(async () => { await mountBoardSketches(host, readers({ readCached: async () => source() })); });
    const cache = spy.mock.calls.find(([event]) => event === 'cache.read');
    const remote = spy.mock.calls.find(([event]) => event === 'remote.read');
    expect(cache[1]).toMatchObject({ status: 'sketch', origin: 'cache', diagram_id: 'b_0123abcd' });
    expect(typeof cache[1].ms).toBe('number');
    expect(typeof remote[1].ms).toBe('number');
    expect(JSON.stringify(spy.mock.calls)).not.toContain('<p>Cached</p>');
    expect(JSON.stringify(spy.mock.calls)).not.toContain(PAGE);
    spy.mockRestore();
  });
});
