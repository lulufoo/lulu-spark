import { createModuleStore } from '../../shared/module-store.ts';
import type { SyncDocumentsResult, SyncProgressEntry } from '../../host/api.ts';

/**
 * How one round of sync ended. A round that could not start is a round like any other: the user
 * is told it ended, and why, in one line, with no list of documents.
 */
export type SyncRunResult =
  /** The host made a sync; part of it may still have gone wrong or been cancelled (see `summary`). */
  | { kind: 'finished'; summary: SyncDocumentsResult }
  /** Nothing was sent to the cloud: not signed in, sign-in refused, cloud out of reach, or the local database. */
  | { kind: 'could_not_start'; reason: string }
  /** The user cancelled while the sign-in was being refreshed: the host was never asked. */
  | { kind: 'cancelled_before_start' };

/**
 * Where the round of sync is. `starting` is the sign-in being refreshed, before the host is asked;
 * `cancelling` goes on until the document being transferred is done.
 */
export type SyncRun =
  | { phase: 'idle' }
  | { phase: 'starting' }
  | { phase: 'running' }
  | { phase: 'cancelling' }
  | { phase: 'ended'; result: SyncRunResult };

/**
 * Whether there is something to sync, from the read-only check. A check that could not be made is
 * `none`: it is neither shown nor told.
 */
export type SyncCheck =
  | { phase: 'unchecked' }
  | { phase: 'checking' }
  | { phase: 'pending'; uploads: number; pulls: number }
  | { phase: 'none' };

/** Whether the user has opened the result of the round that ended last. */
export type SyncSeen = 'unseen' | 'seen';

/** The documents of the round, as the host last listed them. Replaced as a whole, never patched. */
export type SyncRoster = { entries: SyncProgressEntry[] };

export const EMPTY_ROSTER: SyncRoster = { entries: [] };

export const syncRunStore = createModuleStore<SyncRun>({ phase: 'idle' });
export const syncCheckStore = createModuleStore<SyncCheck>({ phase: 'unchecked' });
export const syncSeenStore = createModuleStore<SyncSeen>('seen');
export const syncRosterStore = createModuleStore<SyncRoster>(EMPTY_ROSTER);
