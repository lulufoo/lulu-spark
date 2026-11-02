import { describe, expect, it } from 'vitest';
import { foldProgress, isBusy, nextRun } from '../../frontend/src/home/state/sync-entry-flow.ts';

const idle = { phase: 'idle' };
const starting = { phase: 'starting' };
const running = { phase: 'running' };
const cancelling = { phase: 'cancelling' };
const finished = { kind: 'finished', summary: { ok: true, uploaded: 1, cancelled: false } };
const endedFinished = { phase: 'ended', result: finished };
const endedCancelledEarly = { phase: 'ended', result: { kind: 'cancelled_before_start' } };

const start = { type: 'start' };
const tookOver = { type: 'host_took_over' };
const refused = { type: 'login_refused', reason: 'Sign in first to sync.' };
const cancel = { type: 'cancel' };
const hostResult = { type: 'host_result', result: finished };

describe('isBusy · a round is going on from the first press to the end', () => {
  it.each([
    [idle, false],
    [starting, true],
    [running, true],
    [cancelling, true],
    [endedFinished, false],
  ])('%j → %s', (run, busy) => {
    expect(isBusy(run)).toBe(busy);
  });
});

describe('nextRun · the flow of a round', () => {
  const table = [
    ['idle + start', idle, start, starting],
    ['ended + start', endedFinished, start, starting],
    ['starting + host takes over', starting, tookOver, running],
    [
      'starting + sign-in refused',
      starting,
      refused,
      { phase: 'ended', result: { kind: 'could_not_start', reason: 'Sign in first to sync.' } },
    ],
    ['starting + cancel', starting, cancel, endedCancelledEarly],
    ['running + cancel', running, cancel, cancelling],
    ['running + host answers', running, hostResult, endedFinished],
    ['cancelling + host answers', cancelling, hostResult, endedFinished],
  ];

  it.each(table)('%s', (_name, from, event, to) => {
    expect(nextRun(from, event)).toEqual(to);
  });

  it('anything else leaves the round as it is, the very same object', () => {
    const named = new Set(table.map(([, from, event]) => `${from.phase}/${event.type}`));
    const phases = [idle, starting, running, cancelling, endedFinished];
    const events = [start, tookOver, refused, cancel, hostResult];
    let left = 0;
    for (const from of phases) {
      for (const event of events) {
        if (named.has(`${from.phase}/${event.type}`)) continue;
        expect(nextRun(from, event), `${from.phase} + ${event.type}`).toBe(from);
        left += 1;
      }
    }
    expect(left).toBe(phases.length * events.length - table.length);
  });

  it('a second press of Start, or a late answer, cannot move a round that is going on', () => {
    expect(nextRun(running, start)).toBe(running);
    expect(nextRun(cancelling, start)).toBe(cancelling);
    expect(nextRun(endedCancelledEarly, hostResult)).toBe(endedCancelledEarly);
  });
});

describe('foldProgress · the list the host reads out, folded into the one the screen has', () => {
  const entry = (n, state, extra = {}) => ({
    id: `d${n}`,
    name: `doc-${n}.md`,
    op: 'upload',
    state,
    reason: null,
    local: true,
    ...extra,
  });
  const read = (isRunning, entries) => ({ ok: true, running: isRunning, entries });
  const before = { entries: [entry(1, 'running'), entry(2, 'not_started')] };

  it('replaces the list as a whole, not entry by entry', () => {
    const next = foldProgress(before, read(true, [entry(1, 'done'), entry(2, 'running'), entry(3, 'not_started')]), true);

    expect(next.entries.map((e) => [e.id, e.state])).toEqual([
      ['d1', 'done'],
      ['d2', 'running'],
      ['d3', 'not_started'],
    ]);
  });

  it('is the very same object when nothing changed, so nothing is painted again', () => {
    const same = read(true, [entry(1, 'running'), entry(2, 'not_started')]);

    expect(foldProgress(before, same, true)).toBe(before);
  });

  it.each([
    ['a state', entry(1, 'done')],
    ['a name', entry(1, 'running', { name: 'renamed.md' })],
    ['a reason', entry(1, 'running', { reason: 'could not pack' })],
    ['whether the document is here', entry(1, 'running', { local: false })],
    ['the direction', entry(1, 'running', { op: 'pull' })],
  ])('is a new object when %s changed', (_what, changed) => {
    const next = foldProgress(before, read(true, [changed, entry(2, 'not_started')]), true);

    expect(next).not.toBe(before);
    expect(next.entries[0]).toEqual(changed);
  });

  it('takes an emptied list too: the host starts a round with nothing in it', () => {
    expect(foldProgress(before, read(true, []), true)).toEqual({ entries: [] });
  });

  it('leaves out a record that is not being written while the round goes on', () => {
    const lastRoundsRecord = read(false, [entry(7, 'done'), entry(8, 'failed', { reason: 'x' })]);

    expect(foldProgress(before, lastRoundsRecord, true)).toBe(before);
  });

  it('takes the record as it is once the round has ended: that is the end of the round', () => {
    const end = read(false, [entry(1, 'done'), entry(2, 'failed', { reason: 'could not pack document d2' })]);

    expect(foldProgress(before, end, false)).toEqual({ entries: end.entries });
  });
});
