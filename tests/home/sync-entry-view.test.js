import { describe, expect, it } from 'vitest';
import { deriveSyncEntry } from '../../frontend/src/home/state/sync-entry-view.ts';

const idle = { phase: 'idle' };
const ended = { phase: 'ended', result: { kind: 'finished', summary: { ok: true } } };
const none = { phase: 'none' };
const pending = { phase: 'pending', uploads: 2, pulls: 0 };

const INERT = { dot: false, spinning: false, enabled: true };

describe('deriveSyncEntry · the seven states', () => {
  const states = [
    [
      '1 · started, nothing to sync',
      [idle, none, 'seen'],
      { icon: 'none', start: INERT, view: { dot: false, enabled: false } },
    ],
    [
      '2 · started, something to sync',
      [idle, pending, 'seen'],
      { icon: 'dot', start: { ...INERT, dot: true }, view: { dot: false, enabled: false } },
    ],
    [
      '3 · a sync goes on',
      [{ phase: 'running' }, none, 'seen'],
      {
        icon: 'pulse',
        start: { dot: false, spinning: true, enabled: false },
        view: { dot: false, enabled: true },
      },
    ],
    [
      '4 · just ended, all went through',
      [ended, none, 'unseen'],
      { icon: 'dot', start: INERT, view: { dot: true, enabled: true } },
    ],
    [
      '5 · just ended, some failed',
      [ended, pending, 'unseen'],
      { icon: 'dot', start: { ...INERT, dot: true }, view: { dot: true, enabled: true } },
    ],
    [
      '6 · result seen, still something to sync',
      [ended, pending, 'seen'],
      { icon: 'dot', start: { ...INERT, dot: true }, view: { dot: false, enabled: true } },
    ],
    [
      '7 · result seen, nothing to sync',
      [ended, none, 'seen'],
      { icon: 'none', start: INERT, view: { dot: false, enabled: true } },
    ],
  ];

  it.each(states)('%s', (_name, [run, check, seen], view) => {
    expect(deriveSyncEntry(run, check, seen)).toEqual(view);
  });
});

describe('deriveSyncEntry · what the table leaves out', () => {
  it.each(['starting', 'running', 'cancelling'])('a round that is %s is a sync going on (state 3)', (phase) => {
    for (const check of [none, pending, { phase: 'checking' }]) {
      for (const seen of ['seen', 'unseen']) {
        expect(deriveSyncEntry({ phase }, check, seen)).toEqual({
          icon: 'pulse',
          start: { dot: false, spinning: true, enabled: false },
          view: { dot: false, enabled: true },
        });
      }
    }
  });

  it('a check that is under way or was never made shows no dot, like one that found nothing', () => {
    for (const check of [{ phase: 'unchecked' }, { phase: 'checking' }]) {
      expect(deriveSyncEntry(idle, check, 'seen')).toEqual(deriveSyncEntry(idle, none, 'seen'));
      expect(deriveSyncEntry(ended, check, 'seen')).toEqual(deriveSyncEntry(ended, none, 'seen'));
    }
  });

  it('a check that could not be made is "nothing to sync": no dot, nothing told', () => {
    // `none` is what the command stores for a check that failed; there is no other value to show.
    expect(deriveSyncEntry(idle, none, 'seen').icon).toBe('none');
  });

  it.each([
    ['a sync that could not start', { kind: 'could_not_start', reason: 'Sign in first to sync.' }],
    ['a round cancelled before it started', { kind: 'cancelled_before_start' }],
    ['a sync the user cancelled', { kind: 'finished', summary: { ok: true, cancelled: true } }],
  ])('%s is a result like any other: View can be opened, and it has a dot until it is', (_name, result) => {
    const run = { phase: 'ended', result };

    expect(deriveSyncEntry(run, none, 'unseen')).toEqual({
      icon: 'dot',
      start: INERT,
      view: { dot: true, enabled: true },
    });
    expect(deriveSyncEntry(run, none, 'seen').view).toEqual({ dot: false, enabled: true });
  });

  it('before any round has ended, there is no result to see whatever "seen" says', () => {
    expect(deriveSyncEntry(idle, none, 'unseen').view).toEqual({ dot: false, enabled: false });
  });
});
