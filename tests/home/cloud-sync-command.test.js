// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const refreshAuthSession = vi.hoisted(() => vi.fn());
const syncDocuments = vi.hoisted(() => vi.fn());

vi.mock('../../frontend/src/auth/refresh.ts', () => ({
  refreshAuthSession: (...args) => refreshAuthSession(...args),
}));
vi.mock('../../frontend/src/host/api.ts', () => ({
  syncDocuments: (...args) => syncDocuments(...args),
}));

const { resetCloudSync, runCloudSync } = await import('../../frontend/src/home/commands/cloud-sync.ts');
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
  it('says syncing from the start of the call until the answer', async () => {
    const seen = [];
    refreshAuthSession.mockImplementation(async () => {
      seen.push(cloudSyncStore.getSnapshot().phase);
      return { kind: 'ok' };
    });
    syncDocuments.mockImplementation(async () => {
      seen.push(cloudSyncStore.getSnapshot().phase);
      return result();
    });

    await runCloudSync();

    expect(seen).toEqual(['syncing', 'syncing']);
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
  ])('forgets a sync that ended in %s', (_name, view) => {
    cloudSyncStore.set(view);
    resetCloudSync();
    expect(cloudSyncStore.getSnapshot()).toEqual({ phase: 'idle' });
  });

  it('leaves a sync that is going on alone', () => {
    cloudSyncStore.set({ phase: 'syncing' });
    resetCloudSync();
    expect(cloudSyncStore.getSnapshot()).toEqual({ phase: 'syncing' });
  });
});
