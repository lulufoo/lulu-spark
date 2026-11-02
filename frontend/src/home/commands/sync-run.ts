import { refreshAuthSession } from '../../auth/refresh.ts';
import { cancelSyncDocuments, getSyncProgress, syncDocuments } from '../../host/api.ts';
import { foldProgress, isBusy, nextRun, type SyncRunEvent } from '../state/sync-entry-flow.ts';
import { hostRefusalReason, loginRefusalReason } from '../state/sync-reasons.ts';
import {
  EMPTY_ROSTER,
  syncCheckStore,
  syncRosterStore,
  syncRunStore,
  syncSeenStore,
  type SyncRunResult,
} from '../state/sync-entry.ts';
import { checkSyncPending, dropPendingSyncCheck } from './sync-check.ts';

/** How often the host's record of the round is read while the round goes on. */
export const PROGRESS_POLL_MS = 600;

/** Which round is the latest, so that the step of a round that has been cancelled finds it is not. */
let latest = 0;

function move(event: SyncRunEvent): void {
  syncRunStore.set((run) => nextRun(run, event));
}

/**
 * One round of sync on the user's request: make the sign-in fresh, then ask the host to sync, and
 * read the host's record of the round now and then while it goes on.
 *
 * The host reads the session from the vault and does not refresh it, so the refresh happens here,
 * first. When it cannot be done the host is not asked, and the round ends with the reason.
 *
 * Pressing Start while a round is going on does nothing. There is no retry: the user asks again,
 * and the host plans from the versions as they are by then.
 */
export async function startSync(): Promise<void> {
  if (isBusy(syncRunStore.getSnapshot())) return;
  const mine = ++latest;
  dropPendingSyncCheck();
  syncRosterStore.set(EMPTY_ROSTER);
  move({ type: 'start' });

  const refusal = await loginRefusal();
  // The user may have cancelled while the sign-in was refreshed, and started again after that.
  if (mine !== latest || syncRunStore.getSnapshot().phase !== 'starting') return;
  if (refusal !== null) {
    endRound({ type: 'login_refused', reason: refusal });
    return;
  }

  move({ type: 'host_took_over' });
  const polling = pollProgress();
  const result = await askHost();
  polling.stop();
  if (result.kind === 'finished') {
    await readRoster(false);
    if (result.summary.pulled > 0) document.dispatchEvent(new CustomEvent('cta:reload'));
  }
  endRound({ type: 'host_result', result });
}

/**
 * The user asks the round that is going on to stop. Nothing happens when none is.
 *
 * While the sign-in is refreshed the host has not been asked yet, so the round ends at once and
 * the host is not called. After that the host is asked to stop: it finishes the document being
 * transferred, then the sync answers, with `cancelled: true`, and the round ends.
 */
export async function cancelSync(): Promise<void> {
  const run = syncRunStore.getSnapshot();
  if (run.phase === 'starting') {
    endRound({ type: 'cancel' });
    return;
  }
  if (run.phase !== 'running') return;
  move({ type: 'cancel' });
  try {
    await cancelSyncDocuments();
  } catch {
    /* the sync's own answer ends the round */
  }
}

/** The user opened the result of the round: it no longer counts as unseen. */
export function markSyncResultSeen(): void {
  syncSeenStore.set('seen');
}

/** Forget everything about the rounds, e.g. when the user signs out. A round that goes on is left alone. */
export function resetSyncEntry(): void {
  if (isBusy(syncRunStore.getSnapshot())) return;
  dropPendingSyncCheck();
  syncRunStore.set({ phase: 'idle' });
  syncCheckStore.set({ phase: 'unchecked' });
  syncSeenStore.set('seen');
  syncRosterStore.set(EMPTY_ROSTER);
}

/**
 * A round ends: the result is unseen from now on, and the check is made again, because the round
 * changed what is behind.
 */
function endRound(event: SyncRunEvent): void {
  syncSeenStore.set('unseen');
  move(event);
  void checkSyncPending();
}

async function loginRefusal(): Promise<string | null> {
  try {
    const session = await refreshAuthSession();
    return session.kind === 'ok' ? null : loginRefusalReason(session);
  } catch (error) {
    return hostRefusalReason(null, messageOf(error));
  }
}

async function askHost(): Promise<SyncRunResult> {
  try {
    return { kind: 'finished', summary: await syncDocuments() };
  } catch (error) {
    const status = (error as { status?: unknown } | null)?.status;
    return {
      kind: 'could_not_start',
      reason: hostRefusalReason(typeof status === 'number' ? status : null, messageOf(error)),
    };
  }
}

function messageOf(error: unknown): string {
  return error instanceof Error && error.message ? error.message : 'Sync failed';
}

/** Reads the host's record every `PROGRESS_POLL_MS` until stopped; an answer that is late is dropped. */
function pollProgress(): { stop: () => void } {
  let going = true;
  void (async () => {
    while (going) {
      await new Promise((resolve) => setTimeout(resolve, PROGRESS_POLL_MS));
      if (going) await readRoster(true, () => going);
    }
  })();
  return {
    stop() {
      going = false;
    },
  };
}

/**
 * Reads the whole record and folds it into the list. A read that fails leaves the list as it is:
 * the next one replaces it anyway.
 */
async function readRoster(during: boolean, stillWanted: () => boolean = () => true): Promise<void> {
  try {
    const read = await getSyncProgress();
    if (stillWanted()) syncRosterStore.set((current) => foldProgress(current, read, during));
  } catch {
    /* keep the list */
  }
}
