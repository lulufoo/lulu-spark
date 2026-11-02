import { describe, expect, it } from 'vitest';
import { cloudSyncStatus } from '../../frontend/src/home/ui/cloud-sync-text.ts';

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

const CANCEL_HINT = 'Cancel stops before the next document; the one being transferred is finished.';

describe('cloudSyncStatus · what the user is told while no sync was made, or one is going on', () => {
  const cases = [
    ['nothing yet', { phase: 'idle' }, null],
    [
      'a sync going on',
      { phase: 'syncing', cancelling: false },
      { tone: 'neutral', title: 'Syncing…', detail: CANCEL_HINT },
    ],
    [
      'a sync that is being cancelled',
      { phase: 'syncing', cancelling: true },
      { tone: 'neutral', title: 'Cancelling…', detail: 'Finishing the document being transferred.' },
    ],
    [
      'a cancel before the host was asked',
      { phase: 'cancelled' },
      { tone: 'neutral', title: 'Sync cancelled', detail: 'Nothing was sent or changed.' },
    ],
    [
      'signed out',
      { phase: 'refused', reason: 'signed_out' },
      { tone: 'warn', title: 'Cannot sync', detail: 'Sign in first to sync.' },
    ],
    [
      'the sign-in service out of reach',
      { phase: 'refused', reason: 'session_network' },
      {
        tone: 'warn',
        title: 'Cannot sync',
        detail: 'Could not reach the sign-in service. Check your connection and try again.',
      },
    ],
    [
      'the sign-in no longer valid',
      { phase: 'refused', reason: 'session_invalid' },
      { tone: 'warn', title: 'Cannot sync', detail: 'Your sign-in is no longer valid. Sign in again to sync.' },
    ],
    [
      'the host refusing the session',
      { phase: 'failed', status: 401, message: 'Cannot sync: not signed in' },
      { tone: 'warn', title: 'Cannot sync', detail: 'Your sign-in was not accepted. Sign in again to sync.' },
    ],
    [
      'a sync already running in the host',
      { phase: 'failed', status: 409, message: 'x' },
      { tone: 'warn', title: 'Cannot sync', detail: 'A sync is already running.' },
    ],
    [
      'the host giving a reason',
      { phase: 'failed', status: 502, message: 'Cannot sync: cloud sync not reachable' },
      { tone: 'warn', title: 'Cannot sync', detail: 'Cannot sync: cloud sync not reachable' },
    ],
    [
      'a failure with no status',
      { phase: 'failed', status: null, message: 'Sync failed' },
      { tone: 'warn', title: 'Cannot sync', detail: 'Sync failed' },
    ],
  ];

  it.each(cases)('%s', (_name, view, expected) => {
    expect(cloudSyncStatus(view)).toEqual(expected);
  });
});

describe('cloudSyncStatus · a sync that was made', () => {
  const cases = [
    ['nothing to do', result(), 'neutral', 'Sync finished', 'Uploaded 0 · Pulled 0 · Unchanged 0'],
    [
      'counts only',
      result({ uploaded: 3, pulled: 2, skipped: 10 }),
      'neutral',
      'Sync finished',
      'Uploaded 3 · Pulled 2 · Unchanged 10',
    ],
    [
      'documents kept or gone are not trouble',
      result({ uploaded: 1, kept: 4, gone: 2 }),
      'neutral',
      'Sync finished',
      'Uploaded 1 · Pulled 0 · Unchanged 0',
    ],
    [
      'one document failed',
      result({ uploaded: 1, failed: [{ id: 'd1', op: 'upload', error: 'too big' }] }),
      'warn',
      'Synced, with problems',
      'Uploaded 1 · Pulled 0 · Unchanged 0 · 1 failed · first: too big',
    ],
    [
      'several documents failed: the first reason is shown',
      result({
        failed: [
          { id: 'd1', op: 'pull', error: 'bad zip' },
          { id: 'd2', op: 'upload', error: 'too big' },
        ],
      }),
      'warn',
      'Synced, with problems',
      'Uploaded 0 · Pulled 0 · Unchanged 0 · 2 failed · first: bad zip',
    ],
    [
      'the run stopped early',
      result({
        uploaded: 1,
        not_tried: 4,
        stopped: { id: 'd9', op: 'upload', error: 'cloud sync not reachable' },
      }),
      'warn',
      'Sync stopped',
      'Uploaded 1 · Pulled 0 · Unchanged 0 · stopped early: cloud sync not reachable · 4 not tried, sync again to continue',
    ],
    [
      'a stop with nothing else to report',
      result({ stopped: { id: 'd1', op: 'upload', error: 'offline' } }),
      'warn',
      'Sync stopped',
      'Uploaded 0 · Pulled 0 · Unchanged 0 · stopped early: offline',
    ],
    [
      'failures and a stop together: the stop is the reason shown',
      result({
        failed: [{ id: 'd1', op: 'upload', error: 'too big' }],
        not_tried: 2,
        stopped: { id: 'd2', op: 'pull', error: 'offline' },
      }),
      'warn',
      'Sync stopped',
      'Uploaded 0 · Pulled 0 · Unchanged 0 · 1 failed · stopped early: offline · 2 not tried, sync again to continue',
    ],
    [
      'documents not tried with no reason given',
      result({ not_tried: 1 }),
      'warn',
      'Sync finished',
      'Uploaded 0 · Pulled 0 · Unchanged 0 · 1 not tried, sync again to continue',
    ],
  ];

  it.each(cases)('%s', (_name, answer, tone, title, detail) => {
    expect(cloudSyncStatus({ phase: 'done', result: answer })).toEqual({ tone, title, detail });
  });
});

describe('cloudSyncStatus · a sync the user cancelled', () => {
  const cases = [
    [
      'before anything went through',
      result({ cancelled: true, not_tried: 5 }),
      'neutral',
      'Uploaded 0 · Pulled 0 · Unchanged 0 · 5 not tried, sync again to continue',
    ],
    [
      'after some documents went through: what was done is still told',
      result({ cancelled: true, uploaded: 2, pulled: 1, skipped: 3, not_tried: 4 }),
      'neutral',
      'Uploaded 2 · Pulled 1 · Unchanged 3 · 4 not tried, sync again to continue',
    ],
    [
      'with a document that failed before the cancel: that is still a warning',
      result({
        cancelled: true,
        uploaded: 1,
        not_tried: 2,
        failed: [{ id: 'd1', op: 'upload', error: 'too big' }],
      }),
      'warn',
      'Uploaded 1 · Pulled 0 · Unchanged 0 · 1 failed · first: too big · 2 not tried, sync again to continue',
    ],
  ];

  it.each(cases)('%s', (_name, answer, tone, detail) => {
    expect(cloudSyncStatus({ phase: 'done', result: answer })).toEqual({
      tone,
      title: 'Sync cancelled',
      detail,
    });
  });
});
