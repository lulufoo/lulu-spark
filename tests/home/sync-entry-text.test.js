import { describe, expect, it } from 'vitest';
import { syncCardText } from '../../frontend/src/home/ui/sync-entry-text.ts';

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

const ended = (answer) => ({ phase: 'ended', result: { kind: 'finished', summary: answer } });

const CANCEL_HINT = 'Cancel stops before the next document; the one being transferred is finished.';

describe('syncCardText · a round that is going on, or ended without a sync being made', () => {
  const cases = [
    ['nothing yet', { phase: 'idle' }, null],
    ['the sign-in being refreshed', { phase: 'starting' }, { tone: 'neutral', title: 'Starting…', detail: 'Checking your sign-in.' }],
    ['a sync going on', { phase: 'running' }, { tone: 'neutral', title: 'Syncing…', detail: CANCEL_HINT }],
    [
      'a sync that is being cancelled',
      { phase: 'cancelling' },
      { tone: 'neutral', title: 'Cancelling…', detail: 'Finishing the document being transferred.' },
    ],
    [
      'a cancel before the host was asked',
      { phase: 'ended', result: { kind: 'cancelled_before_start' } },
      { tone: 'neutral', title: 'Sync cancelled', detail: 'Nothing was sent or changed.' },
    ],
    [
      'a round that could not start: the one line of the reason, nothing else',
      { phase: 'ended', result: { kind: 'could_not_start', reason: 'Sign in first to sync.' } },
      { tone: 'warn', title: 'Cannot sync', detail: 'Sign in first to sync.' },
    ],
  ];

  it.each(cases)('%s', (_name, run, expected) => {
    expect(syncCardText(run)).toEqual(expected);
  });
});

describe('syncCardText · a sync that was made', () => {
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
    expect(syncCardText(ended(answer))).toEqual({ tone, title, detail });
  });
});

describe('syncCardText · a sync the user cancelled', () => {
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
    expect(syncCardText(ended(answer))).toEqual({ tone, title: 'Sync cancelled', detail });
  });
});
