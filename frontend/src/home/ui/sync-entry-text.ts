import type { SyncDocumentsResult } from '../../host/api.ts';
import type { SyncRun } from '../state/sync-entry.ts';

/** What the status card says about the round. */
export type SyncCardText = {
  /** `neutral` for progress and plain results, `warn` for something the user may want to act on. */
  tone: 'neutral' | 'warn';
  /** What state the round is in, in a few words. */
  title: string;
  /** The details under the title: counts, the reason, what to do; `null` when there is none. */
  detail: string | null;
};

/**
 * What the status card says about the round that is going on, or the last one; `null` before any.
 *
 * `failuresListed` is true when the panel names the documents that failed, with their reasons. The
 * summary then says how many failed and leaves the first reason to the list.
 *
 * `documentsListed` is false while the host has no list of documents yet: it makes one only after
 * it has the cloud's list and has compared it with this machine's. Until then no document is being
 * transferred, so the card must not talk of one.
 */
export function syncCardText(
  run: SyncRun,
  failuresListed = false,
  documentsListed = true,
): SyncCardText | null {
  switch (run.phase) {
    case 'idle':
      return null;
    case 'starting':
      return { tone: 'neutral', title: 'Starting…', detail: 'Checking your sign-in.' };
    case 'running':
      return documentsListed
        ? {
            tone: 'neutral',
            title: 'Syncing…',
            detail: 'Cancel stops before the next document; the one being transferred is finished.',
          }
        : {
            tone: 'neutral',
            title: 'Comparing with the cloud…',
            detail: 'Nothing is being transferred yet. Cancel stops the sync before the first document.',
          };
    case 'cancelling':
      return documentsListed
        ? { tone: 'neutral', title: 'Cancelling…', detail: 'Finishing the document being transferred.' }
        : {
            tone: 'neutral',
            title: 'Cancelling…',
            detail: 'Waiting for the cloud to answer; nothing will be sent or changed.',
          };
    case 'ended':
      switch (run.result.kind) {
        case 'cancelled_before_start':
          return { tone: 'neutral', title: 'Sync cancelled', detail: 'Nothing was sent or changed.' };
        case 'could_not_start':
          return { tone: 'warn', title: 'Cannot sync', detail: run.result.reason };
        case 'finished':
          return finished(run.result.summary, failuresListed);
      }
  }
}

function finished(result: SyncDocumentsResult, failuresListed: boolean): SyncCardText {
  const trouble = result.failed.length > 0 || result.stopped !== null || result.not_tried > 0;
  const parts = [counts(result)];
  if (result.failed.length > 0) parts.push(`${result.failed.length} failed`);
  if (result.stopped) {
    parts.push(`stopped early: ${result.stopped.error}`);
  } else if (result.failed.length > 0 && !failuresListed) {
    parts.push(`first: ${result.failed[0].error}`);
  }
  if (result.not_tried > 0) parts.push(`${result.not_tried} not tried, sync again to continue`);
  // A cancel is the user's choice: it warns only for what went wrong before it.
  const warn = result.cancelled ? result.failed.length > 0 : trouble;
  return { tone: warn ? 'warn' : 'neutral', title: title(result), detail: parts.join(' · ') };
}

function title(result: SyncDocumentsResult): string {
  if (result.cancelled) return 'Sync cancelled';
  if (result.stopped) return 'Sync stopped';
  if (result.failed.length > 0) return 'Synced, with problems';
  return 'Sync finished';
}

function counts(result: SyncDocumentsResult): string {
  return `Uploaded ${result.uploaded} · Pulled ${result.pulled} · Unchanged ${result.skipped}`;
}
