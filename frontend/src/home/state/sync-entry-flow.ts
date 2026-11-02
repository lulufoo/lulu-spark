import type { SyncProgressEntry, SyncProgressResult } from '../../host/api.ts';
import type { SyncRoster, SyncRun, SyncRunResult } from './sync-entry.ts';

/** A round is going on: the sign-in is refreshed, the host is syncing, or it is stopping. */
export function isBusy(run: SyncRun): boolean {
  return run.phase === 'starting' || run.phase === 'running' || run.phase === 'cancelling';
}

/** What happens to a round, from the user or from the host. */
export type SyncRunEvent =
  | { type: 'start' }
  /** The sign-in is fresh and the host has been asked to sync. */
  | { type: 'host_took_over' }
  /** The sign-in cannot be made fresh: the host is not asked. */
  | { type: 'login_refused'; reason: string }
  | { type: 'cancel' }
  /** The host answered: the sync ended (the document being transferred was finished first). */
  | { type: 'host_result'; result: SyncRunResult };

/**
 * The flow of a round. Whatever the table does not name leaves the round as it is (the same
 * object), so a late answer cannot move a round that has already moved on.
 *
 * | from | event | to |
 * |---|---|---|
 * | idle, ended | start | starting |
 * | starting | host_took_over | running |
 * | starting | login_refused | ended (could not start) |
 * | starting | cancel | ended (cancelled before start) |
 * | running | cancel | cancelling |
 * | running, cancelling | host_result | ended |
 */
export function nextRun(run: SyncRun, event: SyncRunEvent): SyncRun {
  switch (run.phase) {
    case 'idle':
    case 'ended':
      return event.type === 'start' ? { phase: 'starting' } : run;
    case 'starting':
      switch (event.type) {
        case 'host_took_over':
          return { phase: 'running' };
        case 'login_refused':
          return { phase: 'ended', result: { kind: 'could_not_start', reason: event.reason } };
        case 'cancel':
          return { phase: 'ended', result: { kind: 'cancelled_before_start' } };
        default:
          return run;
      }
    case 'running':
      if (event.type === 'cancel') return { phase: 'cancelling' };
      return event.type === 'host_result' ? { phase: 'ended', result: event.result } : run;
    case 'cancelling':
      return event.type === 'host_result' ? { phase: 'ended', result: event.result } : run;
  }
}

/**
 * Folds what the host's record said into the list the screen has. The list is replaced as a whole,
 * never patched; when nothing changed it is the same object, so nothing is painted again.
 *
 * `during` is true while a round is going on. A record the host says is not being written
 * (`running: false`) is then the last round's, or an answer that came too early, and it is left
 * out. The read after the round has ended takes what it is given: that is the end of the round.
 */
export function foldProgress(current: SyncRoster, read: SyncProgressResult, during: boolean): SyncRoster {
  if (during && !read.running) return current;
  return sameEntries(current.entries, read.entries) ? current : { entries: read.entries };
}

function sameEntries(a: SyncProgressEntry[], b: SyncProgressEntry[]): boolean {
  return a.length === b.length && a.every((entry, at) => sameEntry(entry, b[at]));
}

function sameEntry(a: SyncProgressEntry, b: SyncProgressEntry): boolean {
  return (
    a.id === b.id &&
    a.name === b.name &&
    a.op === b.op &&
    a.state === b.state &&
    a.reason === b.reason &&
    a.local === b.local
  );
}
