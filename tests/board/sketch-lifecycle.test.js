// @vitest-environment jsdom
import { act } from 'react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import {
  invalidateBoardSketches, mountBoardSketches, releaseBoardSketches,
} from '../../frontend/src/board/commands/sketch-links.ts';
import { ALIAS, cleanPages, deferred, flush, identity, page, readers, ready, source } from './sketch-fixtures.js';

beforeEach(cleanPages);
afterEach(cleanPages);

describe('Board attachment lifecycle', () => {
  it('waits for the listener before reading and releases a late subscription after unmount', async () => {
    const host = page();
    const subscribed = deferred();
    let reads = 0;
    let stops = 0;
    let callback;
    const pending = mountBoardSketches(host, readers({
      listenInvalidated: (onEvent) => { callback = onEvent; return subscribed.promise; },
      readCached: async () => { reads += 1; return source(); },
      readRemote: async () => { reads += 1; return source(); },
    }));
    await flush();
    expect(host.querySelector('iframe')).not.toBeNull();
    expect(reads).toBe(0);
    await act(async () => {
      releaseBoardSketches(host);
      subscribed.resolve(() => { stops += 1; });
      await pending;
      callback({ ...identity(), reason: 'forbidden' });
    });
    expect(reads).toBe(0);
    expect(stops).toBe(1);
    expect(host.querySelector('iframe')).toBeNull();
    expect(host.querySelector('a').hidden).toBe(false);
  });

  it('shares a listener until the final live attachment, including link fallbacks, is disposed', async () => {
    const first = page();
    const second = page(ALIAS);
    const transport = readers({ readRemote: async () => source({ status: 'not_sketch' }) });
    await act(async () => {
      await mountBoardSketches(first, transport);
      await mountBoardSketches(second, transport);
    });
    expect(transport.subscriptions).toBe(1);
    expect(transport.stops).toBe(0);
    await act(async () => { releaseBoardSketches(first); });
    expect(transport.stops).toBe(0);
    await act(async () => { releaseBoardSketches(second); });
    expect(transport.stops).toBe(1);
  });

  it('continues both reads when subscribing rejects', async () => {
    const host = page();
    const order = [];
    await act(async () => {
      await mountBoardSketches(host, readers({
        listenInvalidated: async () => { order.push('listen'); throw new Error('event IO'); },
        readCached: async () => { order.push('cache'); return source(); },
        readRemote: async () => { order.push('remote'); return { status: 'network' }; },
      }));
    });
    expect(order).toEqual(['listen', 'cache', 'remote']);
    expect(host.querySelector('iframe')).not.toBeNull();
  });

  it.each(['cache', 'remote'])('drops a late %s success after unmount without mutating other attachments', async (path) => {
    const host = page();
    const other = page(ALIAS);
    const gate = deferred();
    const transport = readers();
    const pending = mountBoardSketches(host, {
      ...transport,
      readCached: path === 'cache' ? () => gate.promise : async () => ({ status: 'cache_miss' }),
      readRemote: path === 'remote' ? () => gate.promise : async () => ({ status: 'network' }),
    });
    await flush();
    await act(async () => {
      releaseBoardSketches(host);
      await mountBoardSketches(other, transport);
      gate.resolve(source({ version: 9, diagram_generation: 100 }));
      await pending;
    });
    expect(host.querySelector('iframe')).toBeNull();
    const frame = other.querySelector('iframe');
    const posted = await ready(frame);
    expect(posted[0].html).toBe('<p>Cached</p>');
  });

  it.each(['cache', 'remote'])('rejects old %s success and invalidation after a PAT change', async (path) => {
    const host = page();
    const gate = deferred();
    let cachedReads = 0;
    let remoteReads = 0;
    const transport = readers({
      readCached: () => {
        cachedReads += 1;
        if (cachedReads === 1) return path === 'cache' ? gate.promise : Promise.resolve(source());
        return Promise.resolve(source({ credential_scope: 'scope-new', generation: 2, html: '<p>New PAT</p>' }));
      },
      readRemote: () => {
        remoteReads += 1;
        if (remoteReads === 1) return path === 'remote' ? gate.promise : Promise.resolve({ status: 'network' });
        return Promise.resolve(source({ credential_scope: 'scope-new', generation: 2, html: '<p>New PAT</p>', diagram_generation: 2 }));
      },
    });
    const pending = mountBoardSketches(host, transport);
    await flush();
    await act(async () => { invalidateBoardSketches(); });
    const frame = host.querySelector('iframe');
    const posted = await ready(frame);
    expect(posted[0].html).toBe('<p>New PAT</p>');
    await act(async () => {
      gate.resolve(source({ html: '<p>Old PAT</p>', version: 9, diagram_generation: 99 }));
      await pending;
      transport.emit({ credential_scope: 'scope-a', generation: 1, reason: 'unauthorized' });
    });
    expect(host.querySelector('iframe')).toBe(frame);
    expect(cachedReads).toBe(2);
    expect(remoteReads).toBe(2);
  });

  it('releases a superseded async subscription without detaching its replacement', async () => {
    const host = page();
    const subscribed = deferred();
    let subscriptions = 0;
    let oldStops = 0;
    let newStops = 0;
    const transport = readers({
      listenInvalidated: async () => {
        subscriptions += 1;
        if (subscriptions === 1) return subscribed.promise;
        return () => { newStops += 1; };
      },
    });
    const old = mountBoardSketches(host, transport);
    await flush();
    await act(async () => {
      await mountBoardSketches(host, transport);
      subscribed.resolve(() => { oldStops += 1; });
      await old;
    });
    expect(oldStops).toBe(1);
    expect(newStops).toBe(0);
    expect(host.querySelector('iframe')).not.toBeNull();
    await act(async () => { releaseBoardSketches(host); });
    expect(newStops).toBe(1);
  });

  it.each([undefined, 0, -1, 1.5, 2147483648, '1'])('does not render malformed source version %s', async (version) => {
    const host = page();
    await act(async () => {
      await mountBoardSketches(host, readers({ readRemote: async () => source({ version }) }));
    });
    expect(host.querySelector('iframe')).toBeNull();
    expect(host.querySelector('a').hidden).toBe(false);
  });
});
