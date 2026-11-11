import { describe, expect, it, vi } from 'vitest';
import { watchNativeFullscreen } from '../../frontend/src/app-shell/ui/native-fullscreen.ts';

describe('native fullscreen state', () => {
  it('tracks native enter and exit notifications and cleans up', async () => {
    let notify;
    const unlisten = vi.fn();
    const update = vi.fn();
    const stop = watchNativeFullscreen({
      isFullscreen: async () => false,
      listen: async (name, callback) => {
        expect(name).toBe('spark-native-fullscreen-changed');
        notify = callback;
        return unlisten;
      },
    }, update);
    await vi.waitFor(() => expect(update).toHaveBeenLastCalledWith(false));
    notify({ payload: true });
    expect(update).toHaveBeenLastCalledWith(true);
    notify({ payload: false });
    expect(update).toHaveBeenLastCalledWith(false);
    stop();
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it('does not apply a stale query after a native notification', async () => {
    let resolveQuery;
    let notify;
    const unlisten = vi.fn();
    const update = vi.fn();
    const stop = watchNativeFullscreen({
      isFullscreen: () => new Promise((resolve) => { resolveQuery = resolve; }),
      listen: async (_name, callback) => { notify = callback; return unlisten; },
    }, update);
    await Promise.resolve();
    notify({ payload: true });
    resolveQuery(false);
    await Promise.resolve();
    expect(update).toHaveBeenCalledOnce();
    expect(update).toHaveBeenLastCalledWith(true);
    stop();
  });

  it('cleans up a delayed subscription and preserves safe inset on query failure', async () => {
    let resolveListener;
    const unlisten = vi.fn();
    const update = vi.fn();
    const stop = watchNativeFullscreen({
      isFullscreen: async () => false,
      listen: () => new Promise((resolve) => { resolveListener = resolve; }),
    }, update);
    stop();
    resolveListener(unlisten);
    await Promise.resolve();
    expect(unlisten).toHaveBeenCalledOnce();
    expect(update).not.toHaveBeenCalled();

    const failed = vi.fn();
    const stopFailed = watchNativeFullscreen({
      isFullscreen: async () => { throw new Error('unavailable'); },
      listen: async () => () => {},
    }, failed);
    await vi.waitFor(() => expect(failed).toHaveBeenLastCalledWith(false));
    stopFailed();
  });
});
