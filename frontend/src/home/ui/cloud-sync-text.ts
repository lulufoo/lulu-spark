import type { SyncDocumentsResult } from '../../host/api.ts';
import type { CloudSyncRefusal, CloudSyncView } from '../state/cloud-sync.ts';

export type CloudSyncStatus = {
  /** `neutral` for progress and plain results, `warn` for something the user may want to act on. */
  tone: 'neutral' | 'warn';
  /** What state the sync is in, in a few words. */
  title: string;
  /** The details under the title: counts, the reason, what to do; `null` when there is none. */
  detail: string | null;
};

/** What the sync button says about the sync that is going on, or the last one; `null` before any. */
export function cloudSyncStatus(view: CloudSyncView): CloudSyncStatus | null {
  switch (view.phase) {
    case 'idle':
      return null;
    case 'syncing':
      return view.cancelling
        ? { tone: 'neutral', title: 'Cancelling…', detail: 'Finishing the document being transferred.' }
        : {
            tone: 'neutral',
            title: 'Syncing…',
            detail: 'Cancel stops before the next document; the one being transferred is finished.',
          };
    case 'cancelled':
      return { tone: 'neutral', title: 'Sync cancelled', detail: 'Nothing was sent or changed.' };
    case 'refused':
      return { tone: 'warn', title: 'Cannot sync', detail: refusal(view.reason) };
    case 'failed':
      return { tone: 'warn', title: 'Cannot sync', detail: failure(view.status, view.message) };
    case 'done':
      return done(view.result);
  }
}

function refusal(reason: CloudSyncRefusal): string {
  switch (reason) {
    case 'signed_out':
      return 'Sign in first to sync.';
    case 'session_network':
      return 'Could not reach the sign-in service. Check your connection and try again.';
    case 'session_invalid':
      return 'Your sign-in is no longer valid. Sign in again to sync.';
  }
}

function failure(status: number | null, message: string): string {
  if (status === 401) return 'Your sign-in was not accepted. Sign in again to sync.';
  if (status === 409) return 'A sync is already running.';
  return message;
}

function done(result: SyncDocumentsResult): CloudSyncStatus {
  const trouble = result.failed.length > 0 || result.stopped !== null || result.not_tried > 0;
  const parts = [counts(result)];
  if (result.failed.length > 0) parts.push(`${result.failed.length} failed`);
  if (result.stopped) {
    parts.push(`stopped early: ${result.stopped.error}`);
  } else if (result.failed.length > 0) {
    parts.push(`first: ${result.failed[0].error}`);
  }
  if (result.not_tried > 0) parts.push(`${result.not_tried} not tried, sync again to continue`);
  // A cancel is the user's choice: it warns only for what went wrong before it.
  const warn = result.cancelled ? result.failed.length > 0 : trouble;
  return { tone: warn ? 'warn' : 'neutral', title: doneTitle(result), detail: parts.join(' · ') };
}

function doneTitle(result: SyncDocumentsResult): string {
  if (result.cancelled) return 'Sync cancelled';
  if (result.stopped) return 'Sync stopped';
  if (result.failed.length > 0) return 'Synced, with problems';
  return 'Sync finished';
}

function counts(result: SyncDocumentsResult): string {
  return `Uploaded ${result.uploaded} · Pulled ${result.pulled} · Unchanged ${result.skipped}`;
}
