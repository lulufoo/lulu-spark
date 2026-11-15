import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const sdk = vi.hoisted(() => ({ listen: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => sdk);

beforeEach(() => {
  vi.resetModules();
  sdk.listen.mockReset();
});

afterEach(() => vi.unstubAllGlobals());

describe('Board cache invalidation events', () => {
  it('uses a no-op subscription outside the desktop runtime', async () => {
    vi.stubGlobal('window', {});
    const { listenBoardCacheInvalidated } = await import('../../frontend/src/host/api.ts');
    const callback = vi.fn();
    const unlisten = await listenBoardCacheInvalidated(callback);
    unlisten();
    expect(sdk.listen).not.toHaveBeenCalled();
    expect(callback).not.toHaveBeenCalled();
  });

  it('forwards scoped payloads and returns the desktop unsubscribe function', async () => {
    const unlisten = vi.fn();
    const listen = vi.fn().mockResolvedValue(unlisten);
    vi.stubGlobal('window', { __TAURI__: { event: { listen } } });
    const { listenBoardCacheInvalidated } = await import('../../frontend/src/host/api.ts');
    const callback = vi.fn();
    expect(await listenBoardCacheInvalidated(callback)).toBe(unlisten);
    const [name, handler] = listen.mock.calls[0];
    expect(name).toBe('board-cache:invalidated');
    const payload = {
      credential_scope: 'opaque-scope', generation: 2,
      diagram_id: 'b_0123abcd', diagram_generation: 4, reason: 'forbidden',
    };
    handler({ payload });
    expect(callback).toHaveBeenCalledExactlyOnceWith(payload);
    expect(sdk.listen).not.toHaveBeenCalled();
  });

  it('uses the installed SDK when only the internal desktop runtime is present', async () => {
    const unlisten = vi.fn();
    sdk.listen.mockResolvedValue(unlisten);
    vi.stubGlobal('window', { __TAURI_INTERNALS__: {} });
    const { listenBoardCacheInvalidated } = await import('../../frontend/src/host/api.ts');
    const callback = vi.fn();
    expect(await listenBoardCacheInvalidated(callback)).toBe(unlisten);
    const [name, handler] = sdk.listen.mock.calls[0];
    expect(name).toBe('board-cache:invalidated');
    const payload = { credential_scope: 'old', generation: 3, reason: 'unauthorized' };
    handler({ payload });
    expect(callback).toHaveBeenCalledExactlyOnceWith(payload);
  });
});
