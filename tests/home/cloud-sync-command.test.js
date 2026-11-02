// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const refreshAuthSession = vi.hoisted(() => vi.fn());
const syncDocuments = vi.hoisted(() => vi.fn());
const cancelSyncDocuments = vi.hoisted(() => vi.fn());

vi.mock('../../frontend/src/auth/refresh.ts', () => ({
  refreshAuthSession: (...args) => refreshAuthSession(...args),
}));
vi.mock('../../frontend/src/host/api.ts', () => ({
  syncDocuments: (...args) => syncDocuments(...args),
  cancelSyncDocuments: (...args) => cancelSyncDocuments(...args),
}));

const { CANCEL_GIVE_UP_MS, CANCEL_RETRY_MS, cancelCloudSync, resetCloudSync, runCloudSync } = await import(
  '../../frontend/src/home/commands/cloud-sync.ts'
);
const { cloudSyncStore } = await import('../../frontend/src/home/state/cloud-sync.ts');

function result(overrides = {}) {
  return {
    ok: true,
    uploaded: 0,
    pulled: 0,
    skipped: 0,
    kept: 0,
    gone: 0,
    not_tried: 0,
    failed: [],
    stopped: null,
    cancelled: false,
    ...overrides,
  };
}

function hostError(message, status) {
  const error = new Error(message);
  if (status !== undefined) error.status = status;
  return error;
}

let reloads;
const onReload = () => {
  reloads += 1;
};

beforeEach(() => {
  refreshAuthSession.mockReset();
  syncDocuments.mockReset();
  cancelSyncDocuments.mockReset();
  cloudSyncStore.set({ phase: 'idle' });
  reloads = 0;
  document.addEventListener('cta:reload', onReload);
});

afterEach(() => {
  document.removeEventListener('cta:reload', onReload);
});

describe('runCloudSync · the session decides whether the host is asked', () => {
  const refusals = [
    ['signed out', { kind: 'signed_out' }, 'signed_out'],
    ['the sign-in service is out of reach', { kind: 'failed', reason: 'network' }, 'session_network'],
    ['the sign-in service refuses the session', { kind: 'failed', reason: 'invalid' }, 'session_invalid'],
  ];

  it.each(refusals)('%s: no sync is asked for and the store says why', async (_name, session, reason) => {
    refreshAuthSession.mockResolvedValue(session);

    await runCloudSync();

    expect(syncDocuments).not.toHaveBeenCalled();
    expect(cloudSyncStore.getSnapshot()).toEqual({ phase: 'refused', reason });
    expect(reloads).toBe(0);
  });

  it('a fresh session: the host is asked, after the refresh and not before', async () => {
    const order = [];
    refreshAuthSession.mockImplementation(async () => {
      order.push('refresh');
      return { kind: 'ok' };
    });
    syncDocuments.mockImplementation(async () => {
      order.push('sync');
      return result({ uploaded: 1 });
    });

    await runCloudSync();

    expect(order).toEqual(['refresh', 'sync']);
  });
});

describe('runCloudSync · what the store says', () => {
  it('says syncing, not yet cancelling, from the start of the call until the answer', async () => {
    const seen = [];
    refreshAuthSession.mockImplementation(async () => {
      seen.push(cloudSyncStore.getSnapshot());
      return { kind: 'ok' };
    });
    syncDocuments.mockImplementation(async () => {
      seen.push(cloudSyncStore.getSnapshot());
      return result();
    });

    await runCloudSync();

    expect(seen).toEqual([
      { phase: 'syncing', cancelling: false },
      { phase: 'syncing', cancelling: false },
    ]);
    expect(cloudSyncStore.getSnapshot().phase).toBe('done');
  });

  it('keeps the whole answer of the host, trouble included', async () => {
    const answer = result({
      uploaded: 2,
      pulled: 1,
      skipped: 5,
      not_tried: 3,
      failed: [{ id: 'd1', op: 'upload', error: 'too big' }],
      stopped: { id: 'd2', op: 'pull', error: 'offline' },
    });
    refreshAuthSession.mockResolvedValue({ kind: 'ok' });
    syncDocuments.mockResolvedValue(answer);

    await runCloudSync();

    expect(cloudSyncStore.getSnapshot()).toEqual({ phase: 'done', result: answer });
  });

  const failures = [
    ['a status and a message', hostError('Cannot sync: not signed in', 401), 401, 'Cannot sync: not signed in'],
    ['a busy host', hostError('A sync is already running', 409), 409, 'A sync is already running'],
    ['an unreachable cloud', hostError('Cannot sync: cloud sync not reachable', 502), 502, 'Cannot sync: cloud sync not reachable'],
    ['no status', hostError('boom'), null, 'boom'],
    ['no message', hostError(''), null, 'Sync failed'],
    ['something that is not an Error', 'plain text', null, 'Sync failed'],
  ];

  it.each(failures)('a host call that throws with %s', async (_name, thrown, status, message) => {
    refreshAuthSession.mockResolvedValue({ kind: 'ok' });
    syncDocuments.mockRejectedValue(thrown);

    await runCloudSync();

    expect(cloudSyncStore.getSnapshot()).toEqual({ phase: 'failed', status, message });
    expect(reloads).toBe(0);
  });

  it('a refresh that throws is a failure, not an unhandled error, and the host is not asked', async () => {
    refreshAuthSession.mockRejectedValue(new Error('vault is locked'));

    await runCloudSync();

    expect(cloudSyncStore.getSnapshot()).toEqual({ phase: 'failed', status: null, message: 'vault is locked' });
    expect(syncDocuments).not.toHaveBeenCalled();
  });
});

describe('runCloudSync · the lists are reloaded when documents came in', () => {
  const cases = [
    ['nothing pulled', result({ uploaded: 4, skipped: 2 }), 0],
    ['one pulled', result({ pulled: 1 }), 1],
    ['many pulled', result({ pulled: 7 }), 1],
    ['pulled, with trouble besides', result({ pulled: 1, failed: [{ id: 'x', op: 'upload', error: 'e' }] }), 1],
  ];

  it.each(cases)('%s', async (_name, answer, expected) => {
    refreshAuthSession.mockResolvedValue({ kind: 'ok' });
    syncDocuments.mockResolvedValue(answer);

    await runCloudSync();

    expect(reloads).toBe(expected);
  });
});

describe('runCloudSync · one at a time', () => {
  it('ignores a request made while one is going on', async () => {
    refreshAuthSession.mockResolvedValue({ kind: 'ok' });
    let finish;
    syncDocuments.mockReturnValue(new Promise((resolve) => { finish = resolve; }));

    const first = runCloudSync();
    await Promise.resolve();
    await runCloudSync();
    finish(result({ uploaded: 1 }));
    await first;

    expect(refreshAuthSession).toHaveBeenCalledTimes(1);
    expect(syncDocuments).toHaveBeenCalledTimes(1);
    expect(cloudSyncStore.getSnapshot().phase).toBe('done');
  });

  it('takes a new request once the last one is over, whatever it ended in', async () => {
    refreshAuthSession.mockResolvedValue({ kind: 'ok' });
    syncDocuments.mockRejectedValueOnce(hostError('boom', 502));
    await runCloudSync();
    syncDocuments.mockResolvedValueOnce(result({ uploaded: 1 }));
    await runCloudSync();

    expect(syncDocuments).toHaveBeenCalledTimes(2);
    expect(cloudSyncStore.getSnapshot()).toMatchObject({ phase: 'done' });
  });
});

describe('resetCloudSync', () => {
  it.each([
    ['done', { phase: 'done', result: result() }],
    ['refused', { phase: 'refused', reason: 'signed_out' }],
    ['failed', { phase: 'failed', status: null, message: 'x' }],
    ['cancelled', { phase: 'cancelled' }],
  ])('forgets a sync that ended in %s', (_name, view) => {
    cloudSyncStore.set(view);
    resetCloudSync();
    expect(cloudSyncStore.getSnapshot()).toEqual({ phase: 'idle' });
  });

  it('leaves a sync that is going on alone', () => {
    cloudSyncStore.set({ phase: 'syncing', cancelling: false });
    resetCloudSync();
    expect(cloudSyncStore.getSnapshot()).toEqual({ phase: 'syncing', cancelling: false });
  });
});

describe('cancelCloudSync · there is nothing to cancel', () => {
  const views = [
    ['nothing yet', { phase: 'idle' }],
    ['a finished sync', { phase: 'done', result: result() }],
    ['a refused sync', { phase: 'refused', reason: 'signed_out' }],
    ['a failed sync', { phase: 'failed', status: null, message: 'x' }],
    ['a cancelled sync', { phase: 'cancelled' }],
  ];

  it.each(views)('%s: the host is not asked and the store is as it was', async (_name, view) => {
    cloudSyncStore.set(view);

    await cancelCloudSync();

    expect(cancelSyncDocuments).not.toHaveBeenCalled();
    expect(cloudSyncStore.getSnapshot()).toEqual(view);
  });
});

describe('cancelCloudSync · a sync that the host is running', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    refreshAuthSession.mockResolvedValue({ kind: 'ok' });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  /** A sync the host has been asked for and has not answered. */
  async function startHostSync() {
    let finish;
    syncDocuments.mockReturnValue(new Promise((resolve) => { finish = resolve; }));
    const run = runCloudSync();
    await vi.advanceTimersByTimeAsync(0);
    expect(syncDocuments).toHaveBeenCalledTimes(1);
    return { run, finish };
  }

  it('says cancelling at once, asks the host once, and shows the sync as cancelled when it answers so', async () => {
    cancelSyncDocuments.mockResolvedValue({ ok: true, cancelling: true });
    const { run, finish } = await startHostSync();

    await cancelCloudSync();

    expect(cloudSyncStore.getSnapshot()).toEqual({ phase: 'syncing', cancelling: true });
    expect(cancelSyncDocuments).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(CANCEL_GIVE_UP_MS);
    expect(cancelSyncDocuments).toHaveBeenCalledTimes(1);

    const answer = result({ uploaded: 1, not_tried: 2, cancelled: true });
    finish(answer);
    await run;
    expect(cloudSyncStore.getSnapshot()).toEqual({ phase: 'done', result: answer });
  });

  it('reloads the lists when the documents pulled before the cancel came in', async () => {
    cancelSyncDocuments.mockResolvedValue({ ok: true, cancelling: true });
    const { run, finish } = await startHostSync();
    await cancelCloudSync();

    finish(result({ pulled: 2, not_tried: 1, cancelled: true }));
    await run;

    expect(reloads).toBe(1);
  });

  it('asks again until the host takes the cancel, then stops asking', async () => {
    cancelSyncDocuments
      .mockResolvedValueOnce({ ok: true, cancelling: false })
      .mockResolvedValueOnce({ ok: true, cancelling: false })
      .mockResolvedValue({ ok: true, cancelling: true });
    const { run, finish } = await startHostSync();

    const asking = cancelCloudSync();
    await vi.advanceTimersByTimeAsync(CANCEL_RETRY_MS * 2);
    await asking;

    expect(cancelSyncDocuments).toHaveBeenCalledTimes(3);
    await vi.advanceTimersByTimeAsync(CANCEL_RETRY_MS * 10);
    expect(cancelSyncDocuments).toHaveBeenCalledTimes(3);
    finish(result({ cancelled: true }));
    await run;
  });

  it('stops asking once the sync is over, whatever the host said', async () => {
    cancelSyncDocuments.mockResolvedValue({ ok: true, cancelling: false });
    const { run, finish } = await startHostSync();

    const asking = cancelCloudSync();
    await vi.advanceTimersByTimeAsync(CANCEL_RETRY_MS);
    finish(result({ uploaded: 3 }));
    await run;
    await vi.advanceTimersByTimeAsync(CANCEL_RETRY_MS);
    await asking;
    const asked = cancelSyncDocuments.mock.calls.length;
    await vi.advanceTimersByTimeAsync(CANCEL_GIVE_UP_MS);

    expect(cancelSyncDocuments).toHaveBeenCalledTimes(asked);
    expect(cloudSyncStore.getSnapshot()).toMatchObject({ phase: 'done', result: { cancelled: false, uploaded: 3 } });
  });

  it('gives up after a while when the host never takes it', async () => {
    cancelSyncDocuments.mockResolvedValue({ ok: true, cancelling: false });
    const { run, finish } = await startHostSync();

    const asking = cancelCloudSync();
    await vi.advanceTimersByTimeAsync(CANCEL_GIVE_UP_MS + CANCEL_RETRY_MS * 2);
    await asking;
    const asked = cancelSyncDocuments.mock.calls.length;
    await vi.advanceTimersByTimeAsync(CANCEL_GIVE_UP_MS);

    expect(asked).toBeGreaterThan(CANCEL_GIVE_UP_MS / CANCEL_RETRY_MS - 2);
    expect(asked).toBeLessThanOrEqual(CANCEL_GIVE_UP_MS / CANCEL_RETRY_MS + 2);
    expect(cancelSyncDocuments).toHaveBeenCalledTimes(asked);
    finish(result());
    await run;
  });

  it('a host that cannot be asked ends the asking quietly and leaves the sync to answer', async () => {
    cancelSyncDocuments.mockRejectedValue(new Error('no host'));
    const { run, finish } = await startHostSync();

    await expect(cancelCloudSync()).resolves.toBeUndefined();
    await vi.advanceTimersByTimeAsync(CANCEL_RETRY_MS * 5);

    expect(cancelSyncDocuments).toHaveBeenCalledTimes(1);
    expect(cloudSyncStore.getSnapshot()).toEqual({ phase: 'syncing', cancelling: true });
    finish(result({ uploaded: 1 }));
    await run;
    expect(cloudSyncStore.getSnapshot().phase).toBe('done');
  });

  it('asking again while already cancelling does not ask the host again', async () => {
    cancelSyncDocuments.mockResolvedValue({ ok: true, cancelling: true });
    const { run, finish } = await startHostSync();

    await cancelCloudSync();
    await cancelCloudSync();

    expect(cancelSyncDocuments).toHaveBeenCalledTimes(1);
    finish(result({ cancelled: true }));
    await run;
  });

  it('a cancel that was asked of one sync never reaches the next one', async () => {
    cancelSyncDocuments.mockResolvedValue({ ok: true, cancelling: false });
    const first = await startHostSync();
    const asking = cancelCloudSync();
    await vi.advanceTimersByTimeAsync(CANCEL_RETRY_MS);
    first.finish(result({ uploaded: 1 }));
    await first.run;

    syncDocuments.mockReset();
    syncDocuments.mockReturnValue(new Promise(() => {}));
    void runCloudSync();
    await vi.advanceTimersByTimeAsync(0);
    const askedBefore = cancelSyncDocuments.mock.calls.length;
    await vi.advanceTimersByTimeAsync(CANCEL_RETRY_MS * 4);
    await asking;

    expect(cancelSyncDocuments).toHaveBeenCalledTimes(askedBefore);
    expect(cloudSyncStore.getSnapshot()).toEqual({ phase: 'syncing', cancelling: false });
  });
});

describe('cancelCloudSync · before the host has a sync going', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    cancelSyncDocuments.mockResolvedValue({ ok: true, cancelling: false });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  const sessions = [
    ['a fresh session', { kind: 'ok' }],
    ['no session at all', { kind: 'signed_out' }],
    ['a session that could not be refreshed', { kind: 'failed', reason: 'network' }],
  ];

  it.each(sessions)('cancelled while the sign-in is refreshed (%s): the host is never asked to sync', async (_name, session) => {
    let finishRefresh;
    refreshAuthSession.mockReturnValue(new Promise((resolve) => { finishRefresh = resolve; }));
    const run = runCloudSync();
    await vi.advanceTimersByTimeAsync(0);

    const asking = cancelCloudSync();
    await vi.advanceTimersByTimeAsync(CANCEL_RETRY_MS * 2);
    expect(cloudSyncStore.getSnapshot()).toEqual({ phase: 'syncing', cancelling: true });
    finishRefresh(session);
    await run;
    await vi.advanceTimersByTimeAsync(CANCEL_RETRY_MS);
    await asking;

    expect(syncDocuments).not.toHaveBeenCalled();
    expect(cloudSyncStore.getSnapshot()).toEqual({ phase: 'cancelled' });
    const asked = cancelSyncDocuments.mock.calls.length;
    await vi.advanceTimersByTimeAsync(CANCEL_GIVE_UP_MS);
    expect(cancelSyncDocuments).toHaveBeenCalledTimes(asked);
    expect(reloads).toBe(0);
  });

  it('a new sync can be started after one that was cancelled', async () => {
    let finishRefresh;
    refreshAuthSession.mockReturnValueOnce(new Promise((resolve) => { finishRefresh = resolve; }));
    const run = runCloudSync();
    await vi.advanceTimersByTimeAsync(0);
    void cancelCloudSync();
    finishRefresh({ kind: 'ok' });
    await run;
    expect(cloudSyncStore.getSnapshot()).toEqual({ phase: 'cancelled' });

    refreshAuthSession.mockResolvedValue({ kind: 'ok' });
    syncDocuments.mockResolvedValue(result({ uploaded: 1 }));
    await runCloudSync();

    expect(cloudSyncStore.getSnapshot()).toMatchObject({ phase: 'done', result: { uploaded: 1 } });
  });
});
