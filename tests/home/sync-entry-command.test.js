// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const refreshAuthSession = vi.hoisted(() => vi.fn());
const syncDocuments = vi.hoisted(() => vi.fn());
const cancelSyncDocuments = vi.hoisted(() => vi.fn());
const checkSyncDocuments = vi.hoisted(() => vi.fn());
const getSyncProgress = vi.hoisted(() => vi.fn());

vi.mock('../../frontend/src/auth/refresh.ts', () => ({
  refreshAuthSession: (...args) => refreshAuthSession(...args),
}));
vi.mock('../../frontend/src/host/api.ts', () => ({
  syncDocuments: (...args) => syncDocuments(...args),
  cancelSyncDocuments: (...args) => cancelSyncDocuments(...args),
  checkSyncDocuments: (...args) => checkSyncDocuments(...args),
  getSyncProgress: (...args) => getSyncProgress(...args),
}));

const { PROGRESS_POLL_MS, cancelSync, markSyncResultSeen, resetSyncEntry, startSync } = await import(
  '../../frontend/src/home/commands/sync-run.ts'
);
const { checkSyncPending, dropPendingSyncCheck } = await import('../../frontend/src/home/commands/sync-check.ts');
const { syncCheckStore, syncRosterStore, syncRunStore, syncSeenStore } = await import(
  '../../frontend/src/home/state/sync-entry.ts'
);

function summary(overrides = {}) {
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

function entry(n, state, extra = {}) {
  return { id: `d${n}`, name: `doc-${n}.md`, op: 'upload', state, reason: null, local: true, ...extra };
}

function record(running, entries) {
  return { ok: true, running, entries };
}

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function hostError(message, status) {
  const error = new Error(message);
  if (status !== undefined) error.status = status;
  return error;
}

const run = () => syncRunStore.getSnapshot();
const check = () => syncCheckStore.getSnapshot();
const seen = () => syncSeenStore.getSnapshot();
const roster = () => syncRosterStore.getSnapshot().entries;

let reloads;
const onReload = () => {
  reloads += 1;
};

beforeEach(() => {
  vi.useFakeTimers();
  for (const mock of [refreshAuthSession, syncDocuments, cancelSyncDocuments, checkSyncDocuments, getSyncProgress]) {
    mock.mockReset();
  }
  refreshAuthSession.mockResolvedValue({ kind: 'ok' });
  syncDocuments.mockResolvedValue(summary());
  cancelSyncDocuments.mockResolvedValue({ ok: true, cancelling: true });
  checkSyncDocuments.mockResolvedValue({ ok: true, uploads: 0, pulls: 0 });
  getSyncProgress.mockResolvedValue(record(false, []));
  syncRunStore.set({ phase: 'idle' });
  syncCheckStore.set({ phase: 'unchecked' });
  syncSeenStore.set('seen');
  syncRosterStore.set({ entries: [] });
  reloads = 0;
  document.addEventListener('cta:reload', onReload);
});

afterEach(async () => {
  document.removeEventListener('cta:reload', onReload);
  await vi.runOnlyPendingTimersAsync();
  vi.useRealTimers();
});

/** Lets what is waiting on a promise run, without moving the clock. */
const settle = () => vi.advanceTimersByTimeAsync(0);

describe('startSync · a round', () => {
  it('refreshes the sign-in, then asks the host; reads the list while it goes on, and once more after', async () => {
    const order = [];
    refreshAuthSession.mockImplementation(async () => {
      order.push('refresh');
      return { kind: 'ok' };
    });
    const host = deferred();
    syncDocuments.mockImplementation(() => {
      order.push('sync');
      return host.promise;
    });
    getSyncProgress
      .mockResolvedValueOnce(record(true, [entry(1, 'running'), entry(2, 'not_started')]))
      .mockResolvedValueOnce(record(true, [entry(1, 'done'), entry(2, 'running')]))
      .mockResolvedValueOnce(record(false, [entry(1, 'done'), entry(2, 'done')]));

    const started = startSync();
    expect(run()).toEqual({ phase: 'starting' });
    await settle();
    expect(run()).toEqual({ phase: 'running' });
    expect(order).toEqual(['refresh', 'sync']);
    expect(roster()).toEqual([]);

    await vi.advanceTimersByTimeAsync(PROGRESS_POLL_MS);
    expect(roster().map((e) => e.state)).toEqual(['running', 'not_started']);
    await vi.advanceTimersByTimeAsync(PROGRESS_POLL_MS);
    expect(roster().map((e) => e.state)).toEqual(['done', 'running']);

    host.resolve(summary({ uploaded: 2 }));
    await started;

    expect(getSyncProgress).toHaveBeenCalledTimes(3);
    expect(roster().map((e) => e.state)).toEqual(['done', 'done']);
    expect(run()).toEqual({ phase: 'ended', result: { kind: 'finished', summary: summary({ uploaded: 2 }) } });
    expect(seen()).toBe('unseen');
    expect(reloads).toBe(0);
  });

  it('forgets the last round’s list as soon as the next one starts', async () => {
    syncRosterStore.set({ entries: [entry(9, 'done')] });
    refreshAuthSession.mockReturnValue(deferred().promise);

    void startSync();

    expect(roster()).toEqual([]);
  });

  it('asks the host to sync only once when Start is pressed again during a round', async () => {
    const host = deferred();
    syncDocuments.mockReturnValue(host.promise);

    const first = startSync();
    await startSync();
    await settle();
    await startSync();
    host.resolve(summary());
    await first;

    expect(syncDocuments).toHaveBeenCalledTimes(1);
  });

  it('tells the page to reload when documents came down', async () => {
    syncDocuments.mockResolvedValue(summary({ pulled: 2 }));

    await startSync();

    expect(reloads).toBe(1);
  });

  it('leaves out a record the host says is not being written, and keeps the end of the round', async () => {
    const host = deferred();
    syncDocuments.mockReturnValue(host.promise);
    getSyncProgress
      .mockResolvedValueOnce(record(false, [entry(7, 'done')])) // the last round's, read before the host began
      .mockResolvedValueOnce(record(false, [entry(1, 'done'), entry(2, 'failed', { reason: 'could not pack' })]));

    const started = startSync();
    await vi.advanceTimersByTimeAsync(PROGRESS_POLL_MS);
    expect(roster()).toEqual([]);

    host.resolve(summary({ uploaded: 1, failed: [{ id: 'd2', op: 'upload', error: 'could not pack' }] }));
    await started;

    expect(roster().map((e) => [e.id, e.state])).toEqual([
      ['d1', 'done'],
      ['d2', 'failed'],
    ]);
  });

  it('drops a read that comes back after the round has been read for the last time', async () => {
    const host = deferred();
    const late = deferred();
    syncDocuments.mockReturnValue(host.promise);
    getSyncProgress
      .mockReturnValueOnce(late.promise)
      .mockResolvedValueOnce(record(false, [entry(1, 'done')]));

    const started = startSync();
    await vi.advanceTimersByTimeAsync(PROGRESS_POLL_MS);
    host.resolve(summary({ uploaded: 1 }));
    await started;
    late.resolve(record(true, [entry(1, 'running')]));
    await settle();

    expect(roster().map((e) => e.state)).toEqual(['done']);
  });

  it('keeps the list it has when a read fails', async () => {
    const host = deferred();
    syncDocuments.mockReturnValue(host.promise);
    getSyncProgress
      .mockResolvedValueOnce(record(true, [entry(1, 'running')]))
      .mockRejectedValueOnce(new Error('the host did not answer'))
      .mockRejectedValueOnce(new Error('the host did not answer'));

    const started = startSync();
    await vi.advanceTimersByTimeAsync(PROGRESS_POLL_MS);
    await vi.advanceTimersByTimeAsync(PROGRESS_POLL_MS);
    host.resolve(summary({ uploaded: 1 }));
    await started;

    expect(roster().map((e) => e.state)).toEqual(['running']);
    expect(run().phase).toBe('ended');
  });
});

describe('startSync · a round that could not start is a round like any other', () => {
  const refusals = [
    ['signed out', { kind: 'signed_out' }, 'Sign in first to sync.'],
    [
      'the sign-in service is out of reach',
      { kind: 'failed', reason: 'network' },
      'Could not reach the sign-in service. Check your connection and try again.',
    ],
    [
      'the sign-in service refuses the session',
      { kind: 'failed', reason: 'invalid' },
      'Your sign-in is no longer valid. Sign in again to sync.',
    ],
  ];

  it.each(refusals)('%s: the host is not asked, and the round ends with one line and no list', async (_n, session, reason) => {
    refreshAuthSession.mockResolvedValue(session);

    await startSync();

    expect(syncDocuments).not.toHaveBeenCalled();
    expect(getSyncProgress).not.toHaveBeenCalled();
    expect(run()).toEqual({ phase: 'ended', result: { kind: 'could_not_start', reason } });
    expect(roster()).toEqual([]);
    expect(seen()).toBe('unseen');
    expect(reloads).toBe(0);
  });

  it('a sign-in refresh that throws is the same: the host is not asked', async () => {
    refreshAuthSession.mockRejectedValueOnce(new Error('vault unavailable'));

    await startSync();

    expect(syncDocuments).not.toHaveBeenCalled();
    expect(run()).toEqual({ phase: 'ended', result: { kind: 'could_not_start', reason: 'vault unavailable' } });
  });

  it.each([
    [401, 'Cannot sync: not signed in', 'Your sign-in was not accepted. Sign in again to sync.'],
    [409, 'Cannot sync: a sync is already running', 'A sync is already running.'],
    [502, 'Cannot sync: the cloud cannot be used', 'Cannot sync: the cloud cannot be used'],
    [500, 'Cannot sync: could not read the document database', 'Cannot sync: could not read the document database'],
    [undefined, 'fetch failed', 'fetch failed'],
  ])('the host answers that no sync was made (%s): the round ends with one line', async (status, message, reason) => {
    syncDocuments.mockRejectedValueOnce(hostError(message, status));

    await startSync();

    expect(run()).toEqual({ phase: 'ended', result: { kind: 'could_not_start', reason } });
    expect(roster()).toEqual([]);
    expect(seen()).toBe('unseen');
  });

  it('an error with no words of its own gets a plain line', async () => {
    syncDocuments.mockRejectedValueOnce({ status: 502 });

    await startSync();

    expect(run()).toEqual({ phase: 'ended', result: { kind: 'could_not_start', reason: 'Sync failed' } });
  });
});

describe('cancelSync', () => {
  it('while the sign-in is refreshed: the round ends at once, and the host is never asked', async () => {
    const refresh = deferred();
    refreshAuthSession.mockReturnValueOnce(refresh.promise);

    const started = startSync();
    await cancelSync();

    expect(run()).toEqual({ phase: 'ended', result: { kind: 'cancelled_before_start' } });
    expect(seen()).toBe('unseen');

    refresh.resolve({ kind: 'ok' });
    await started;
    await settle();

    expect(syncDocuments).not.toHaveBeenCalled();
    expect(cancelSyncDocuments).not.toHaveBeenCalled();
    expect(run()).toEqual({ phase: 'ended', result: { kind: 'cancelled_before_start' } });
  });

  it('a refresh that was cancelled and answers late cannot start the round that came after it', async () => {
    const first = deferred();
    const second = deferred();
    refreshAuthSession
      .mockReturnValueOnce(first.promise) // Start
      .mockResolvedValueOnce({ kind: 'ok' }) // the check after the cancelled round
      .mockReturnValueOnce(second.promise); // Start again
    const host = deferred();
    syncDocuments.mockReturnValue(host.promise);

    const firstStart = startSync();
    await cancelSync();
    await settle();
    const secondStart = startSync();
    first.resolve({ kind: 'ok' });
    await firstStart;
    await settle();

    expect(syncDocuments).not.toHaveBeenCalled();
    expect(run()).toEqual({ phase: 'starting' });

    second.resolve({ kind: 'ok' });
    await settle();
    expect(syncDocuments).toHaveBeenCalledTimes(1);
    host.resolve(summary());
    await secondStart;
    expect(run().phase).toBe('ended');
  });

  it('while the host syncs: asks the host once and waits for its answer, which ends the round', async () => {
    const host = deferred();
    syncDocuments.mockReturnValue(host.promise);
    const started = startSync();
    await settle();

    await cancelSync();
    await cancelSync();

    expect(run()).toEqual({ phase: 'cancelling' });
    expect(cancelSyncDocuments).toHaveBeenCalledTimes(1);

    const answer = summary({ uploaded: 1, not_tried: 2, cancelled: true });
    host.resolve(answer);
    await started;

    expect(run()).toEqual({ phase: 'ended', result: { kind: 'finished', summary: answer } });
    expect(seen()).toBe('unseen');
  });

  it('does not stop the round when the cancel call itself fails: the host’s answer still ends it', async () => {
    const host = deferred();
    syncDocuments.mockReturnValue(host.promise);
    cancelSyncDocuments.mockRejectedValueOnce(new Error('ipc closed'));
    const started = startSync();
    await settle();

    await cancelSync();
    host.resolve(summary());
    await started;

    expect(run().phase).toBe('ended');
  });

  it('does nothing when no round is going on', async () => {
    await cancelSync();
    expect(run()).toEqual({ phase: 'idle' });

    await startSync();
    const ended = run();
    await cancelSync();

    expect(run()).toBe(ended);
    expect(cancelSyncDocuments).not.toHaveBeenCalled();
  });
});

describe('checkSyncPending · is there anything to sync', () => {
  it.each([
    [{ ok: true, uploads: 2, pulls: 0 }, { phase: 'pending', uploads: 2, pulls: 0 }],
    [{ ok: true, uploads: 0, pulls: 3 }, { phase: 'pending', uploads: 0, pulls: 3 }],
    [{ ok: true, uploads: 1, pulls: 1 }, { phase: 'pending', uploads: 1, pulls: 1 }],
    [{ ok: true, uploads: 0, pulls: 0 }, { phase: 'none' }],
  ])('the host counts %j → %j', async (counts, expected) => {
    checkSyncDocuments.mockResolvedValue(counts);

    await checkSyncPending();

    expect(check()).toEqual(expected);
  });

  it('refreshes the sign-in first, and says it is checking while it does', async () => {
    const order = [];
    const refresh = deferred();
    refreshAuthSession.mockImplementation(() => {
      order.push('refresh');
      return refresh.promise;
    });
    checkSyncDocuments.mockImplementation(async () => {
      order.push('check');
      return { ok: true, uploads: 1, pulls: 0 };
    });

    const checking = checkSyncPending();
    expect(check()).toEqual({ phase: 'checking' });
    refresh.resolve({ kind: 'ok' });
    await checking;

    expect(order).toEqual(['refresh', 'check']);
    expect(check().phase).toBe('pending');
  });

  it('asks the cloud nothing when nobody is signed in', async () => {
    refreshAuthSession.mockResolvedValue({ kind: 'signed_out' });

    await checkSyncPending();

    expect(checkSyncDocuments).not.toHaveBeenCalled();
    expect(check()).toEqual({ phase: 'none' });
  });

  it.each([
    ['the sign-in cannot be made fresh (network)', () => refreshAuthSession.mockResolvedValue({ kind: 'failed', reason: 'network' })],
    ['the sign-in is refused', () => refreshAuthSession.mockResolvedValue({ kind: 'failed', reason: 'invalid' })],
    ['the refresh throws', () => refreshAuthSession.mockRejectedValue(new Error('vault unavailable'))],
    ['the host fails', () => checkSyncDocuments.mockRejectedValue(hostError('Cannot sync: cloud', 502))],
    ['the host says the session is not accepted', () => checkSyncDocuments.mockRejectedValue(hostError('Cannot sync', 401))],
  ])('%s: it shows no mark and tells nothing', async (_what, arrange) => {
    arrange();
    syncCheckStore.set({ phase: 'pending', uploads: 1, pulls: 0 });

    await expect(checkSyncPending()).resolves.toBeUndefined();

    expect(check()).toEqual({ phase: 'none' });
    expect(run()).toEqual({ phase: 'idle' });
    expect(seen()).toBe('seen');
  });

  it('is not made while a round is going on', async () => {
    syncRunStore.set({ phase: 'running' });

    await checkSyncPending();

    expect(refreshAuthSession).not.toHaveBeenCalled();
    expect(check()).toEqual({ phase: 'unchecked' });
  });

  it('the latest check wins, whatever order the answers come in', async () => {
    const slow = deferred();
    checkSyncDocuments
      .mockReturnValueOnce(slow.promise)
      .mockResolvedValueOnce({ ok: true, uploads: 0, pulls: 0 });

    const older = checkSyncPending();
    await checkSyncPending();
    slow.resolve({ ok: true, uploads: 5, pulls: 0 });
    await older;

    expect(check()).toEqual({ phase: 'none' });
  });

  it('an answer that was on its way when a round began is dropped', async () => {
    const slow = deferred();
    checkSyncDocuments.mockReturnValueOnce(slow.promise);

    const checking = checkSyncPending();
    await settle();
    dropPendingSyncCheck();
    expect(check()).toEqual({ phase: 'unchecked' });
    slow.resolve({ ok: true, uploads: 5, pulls: 0 });
    await checking;

    expect(check()).toEqual({ phase: 'unchecked' });
  });
});

describe('the check around a round', () => {
  it('is made again after every round, whatever the round’s end', async () => {
    checkSyncDocuments.mockResolvedValue({ ok: true, uploads: 1, pulls: 0 });

    await startSync();
    await settle();

    expect(checkSyncDocuments).toHaveBeenCalledTimes(1);
    expect(check()).toEqual({ phase: 'pending', uploads: 1, pulls: 0 });
  });

  it('finds documents that failed still behind (state 5), and none when all went through (state 4)', async () => {
    checkSyncDocuments.mockResolvedValueOnce({ ok: true, uploads: 5, pulls: 0 });
    await startSync();
    await settle();
    expect(check().phase).toBe('pending');

    checkSyncDocuments.mockResolvedValueOnce({ ok: true, uploads: 0, pulls: 0 });
    await startSync();
    await settle();
    expect(check().phase).toBe('none');
  });

  it('is made after a round that could not start, and shows nothing if it cannot be made either', async () => {
    refreshAuthSession.mockResolvedValue({ kind: 'signed_out' });

    await startSync();
    await settle();

    expect(check()).toEqual({ phase: 'none' });
    expect(checkSyncDocuments).not.toHaveBeenCalled();
  });

  it('is made after a round the user cancelled before it started', async () => {
    const refresh = deferred();
    refreshAuthSession.mockReturnValueOnce(refresh.promise);
    checkSyncDocuments.mockResolvedValue({ ok: true, uploads: 3, pulls: 0 });

    const started = startSync();
    await cancelSync();
    await settle();
    refresh.resolve({ kind: 'ok' });
    await started;

    expect(check()).toEqual({ phase: 'pending', uploads: 3, pulls: 0 });
  });

  it('is not left under way by a round that begins while it is: nothing of it outlives the round', async () => {
    const slow = deferred();
    checkSyncDocuments.mockReturnValueOnce(slow.promise).mockResolvedValue({ ok: true, uploads: 0, pulls: 0 });
    const checking = checkSyncPending();
    await settle();

    await startSync();
    await settle();
    slow.resolve({ ok: true, uploads: 9, pulls: 0 });
    await checking;

    expect(check()).toEqual({ phase: 'none' });
  });
});

describe('markSyncResultSeen and resetSyncEntry', () => {
  it('opening the result marks it seen, and the next round’s end marks it unseen again', async () => {
    await startSync();
    expect(seen()).toBe('unseen');

    markSyncResultSeen();
    expect(seen()).toBe('seen');

    await startSync();
    expect(seen()).toBe('unseen');
  });

  it('forgets the rounds, e.g. at sign-out', async () => {
    await startSync();
    await settle();
    syncRosterStore.set({ entries: [entry(1, 'done')] });

    resetSyncEntry();

    expect(run()).toEqual({ phase: 'idle' });
    expect(check()).toEqual({ phase: 'unchecked' });
    expect(seen()).toBe('seen');
    expect(roster()).toEqual([]);
  });

  it('leaves a round that goes on alone', async () => {
    const host = deferred();
    syncDocuments.mockReturnValue(host.promise);
    const started = startSync();
    await settle();

    resetSyncEntry();

    expect(run()).toEqual({ phase: 'running' });
    host.resolve(summary());
    await started;
    expect(run().phase).toBe('ended');
  });
});
