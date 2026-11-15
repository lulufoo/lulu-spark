// @vitest-environment jsdom
import { act } from 'react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { mountBoardSketches } from '../../frontend/src/board/commands/sketch-links.ts';
import {
  ALIAS, OTHER, PAGE, cleanPages, deferred, flush, identity, page, readers, ready, source,
} from './sketch-fixtures.js';

beforeEach(cleanPages);
afterEach(cleanPages);

function denial(options = {}) {
  return { ...identity(), status: 'forbidden', invalidation_scope: 'diagram', reason: 'forbidden', ...options };
}

describe('Board access freshness', () => {
  it('invalidates URL aliases across containers while retaining unrelated graphs and credentials', async () => {
    const first = page();
    const alias = page(ALIAS);
    const other = page(OTHER);
    const anotherScope = page(ALIAS);
    let reads = 0;
    const transport = readers({
      readCached: async (url) => { reads += 1; return source({ url }); },
      readRemote: async () => ({ status: 'network' }),
    });
    await act(async () => {
      await Promise.all([first, alias, other].map((host) => mountBoardSketches(host, transport)));
      await mountBoardSketches(anotherScope, {
        ...transport, readCached: async () => source({ credential_scope: 'scope-b' }),
      });
    });
    expect(transport.subscriptions).toBe(1);
    await act(async () => { transport.emit(denial({ diagram_generation: 2 })); });
    expect(first.querySelector('iframe')).toBeNull();
    expect(alias.querySelector('iframe')).toBeNull();
    expect(other.querySelector('iframe')).not.toBeNull();
    expect(anotherScope.querySelector('iframe')).not.toBeNull();
    await act(async () => {
      transport.emit({ credential_scope: 'scope-a', generation: 2, reason: 'unauthorized' });
    });
    expect(other.querySelector('iframe')).toBeNull();
    expect(anotherScope.querySelector('iframe')).not.toBeNull();
    expect(reads).toBe(3);
  });

  it.each(['forbidden', 'not_found'])('fans out the remote %s scope without starting new reads', async (status) => {
    const first = page();
    const second = page(ALIAS);
    const other = page(OTHER);
    const remote = deferred();
    let reads = 0;
    const transport = readers({
      readCached: async (url) => source({ url }),
      readRemote: (url) => {
        reads += 1;
        return url === PAGE ? remote.promise : Promise.resolve({ status: 'network' });
      },
    });
    const pending = Promise.all([first, second, other].map((host) => mountBoardSketches(host, transport)));
    await flush();
    await act(async () => { remote.resolve(denial({ status, diagram_generation: 2 })); await pending; });
    expect(first.querySelector('iframe')).toBeNull();
    expect(second.querySelector('iframe')).toBeNull();
    expect(other.querySelector('iframe')).not.toBeNull();
    expect(reads).toBe(3);
  });

  it('ignores a late old 403 reply and event after same-version permission recovery', async () => {
    const old = page();
    const recovered = page(ALIAS);
    const remote = deferred();
    const transport = readers({ readCached: async (url) => source({ url }), readRemote: () => remote.promise });
    const pending = mountBoardSketches(old, transport);
    await flush();
    await act(async () => {
      await mountBoardSketches(recovered, {
        ...transport, readRemote: async () => source({ diagram_generation: 3 }),
      });
    });
    const frame = recovered.querySelector('iframe');
    await act(async () => {
      remote.resolve(denial({ diagram_generation: 2 }));
      await pending;
      transport.emit(denial({ diagram_generation: 2 }));
    });
    expect(recovered.querySelector('iframe')).toBe(frame);
    expect(old.querySelector('iframe')).not.toBeNull();
  });

  it('does not let a newer network epoch hide an earlier known denial', async () => {
    const host = page();
    const remote = deferred();
    const transport = readers({ readCached: async () => source(), readRemote: () => remote.promise });
    const pending = mountBoardSketches(host, transport);
    await flush();
    await act(async () => {
      remote.resolve({ ...identity(PAGE, { diagram_generation: 3 }), status: 'network' });
      await pending;
      transport.emit(denial({ diagram_generation: 2 }));
    });
    expect(host.querySelector('iframe')).toBeNull();
    expect(host.querySelector('a').hidden).toBe(false);
  });

  it('rejects an older credential scope success after another view confirms a new credential generation', async () => {
    const old = page();
    const current = page(ALIAS);
    const remote = deferred();
    const transport = readers({ readRemote: () => remote.promise });
    const pending = mountBoardSketches(old, transport);
    await flush();
    await act(async () => {
      await mountBoardSketches(current, {
        ...transport,
        readRemote: async () => source({ credential_scope: 'scope-new', generation: 2 }),
      });
      remote.resolve(source({ html: '<p>Old credential</p>', diagram_generation: 50 }));
      await pending;
      transport.emit(denial({ diagram_generation: 100 }));
    });
    expect(old.querySelector('iframe')).toBeNull();
    const posted = await ready(current.querySelector('iframe'));
    expect(posted[0].html).toBe('<p>Cached</p>');
  });

  it('allows a retired credential event to clear only its already displayed old views', async () => {
    const old = page();
    const current = page(ALIAS);
    const transport = readers();
    await act(async () => {
      await mountBoardSketches(old, transport);
      await mountBoardSketches(current, {
        ...transport, readRemote: async () => source({ credential_scope: 'scope-new', generation: 3 }),
      });
    });
    const currentFrame = current.querySelector('iframe');
    expect(old.querySelector('iframe')).not.toBeNull();
    await act(async () => {
      transport.emit({ credential_scope: 'scope-a', generation: 2, reason: 'credential_changed' });
    });
    expect(old.querySelector('iframe')).toBeNull();
    expect(current.querySelector('iframe')).toBe(currentFrame);
  });

  it('keeps a denial when a late old cache and a newer network failure arrive', async () => {
    const host = page();
    const cache = deferred();
    const remote = deferred();
    const transport = readers({ readCached: () => cache.promise, readRemote: () => remote.promise });
    const pending = mountBoardSketches(host, transport);
    await flush();
    await act(async () => {
      transport.emit(denial({ diagram_generation: 2 }));
      cache.resolve(source());
      remote.resolve({ ...identity(PAGE, { diagram_generation: 3 }), status: 'network' });
      await pending;
    });
    expect(host.querySelector('iframe')).toBeNull();
  });

  it('accepts remote recovery when cache-only denial observed the same in-flight epoch', async () => {
    const host = page();
    const remote = deferred();
    const transport = readers({
      readCached: async () => denial({ diagram_generation: 3 }), readRemote: () => remote.promise,
    });
    const pending = mountBoardSketches(host, transport);
    await flush();
    expect(host.querySelector('iframe')).toBeNull();
    await act(async () => { remote.resolve(source({ diagram_generation: 3 })); await pending; });
    expect(host.querySelector('iframe')).not.toBeNull();
  });

  it('does not let a late cache-only denial undo remote recovery in another container', async () => {
    const first = page();
    const alias = page(ALIAS);
    const cache = deferred();
    const transport = readers({ readRemote: async () => source({ diagram_generation: 3 }) });
    await act(async () => { await mountBoardSketches(first, transport); });
    const frame = first.querySelector('iframe');
    const pending = mountBoardSketches(alias, {
      ...transport, readCached: () => cache.promise, readRemote: async () => ({ status: 'network' }),
    });
    await flush();
    await act(async () => { cache.resolve(denial({ diagram_generation: 3 })); await pending; });
    expect(first.querySelector('iframe')).toBe(frame);
  });

  it('still paints a new container cache after an existing same-diagram remote success', async () => {
    const first = page();
    const next = page(ALIAS);
    const remote = deferred();
    const transport = readers({ readCached: async (url) => source({ url }) });
    await act(async () => { await mountBoardSketches(first, transport); });
    const pending = mountBoardSketches(next, { ...transport, readRemote: () => remote.promise });
    await flush();
    const frame = next.querySelector('iframe');
    const posted = await ready(frame);
    expect(posted[0].html).toBe('<p>Cached</p>');
    await act(async () => { remote.resolve({ status: 'network' }); await pending; });
    expect(next.querySelector('iframe')).toBe(frame);
  });

  it.each(['not_sketch', 'invalid_source'])('treats fresh %s as source evidence against an old 403', async (status) => {
    const host = page();
    const transport = readers({ readRemote: async () => source({ status, diagram_generation: 3 }) });
    await act(async () => { await mountBoardSketches(host, transport); });
    await act(async () => { transport.emit(denial({ diagram_generation: 2 })); });
    // The live fallback survives old invalidation and remains subscribed.
    expect(host.querySelector('a').hidden).toBe(false);
    expect(transport.stops).toBe(0);
    const alias = page(ALIAS);
    await act(async () => {
      await mountBoardSketches(alias, {
        ...transport, readCached: async () => source({ diagram_generation: 3 }),
        readRemote: async () => ({ status: 'network' }),
      });
    });
    expect(alias.querySelector('iframe')).not.toBeNull();
  });
});
