import type { SyncDocumentsResult } from '../../host/api.ts';
import type { CloudSyncView } from '../state/cloud-sync.ts';

export type CloudSyncLine = {
  /** `neutral` for progress and plain results, `warn` for something the user may want to act on. */
  tone: 'neutral' | 'warn';
  text: string;
};

/** What the account menu says about the last sync; `null` while there is nothing to say. */
export function cloudSyncLine(view: CloudSyncView): CloudSyncLine | null {
  switch (view.phase) {
    case 'idle':
      return null;
    case 'syncing':
      return { tone: 'neutral', text: 'Syncing…' };
    case 'refused':
      return { tone: 'warn', text: refusal(view.reason) };
    case 'failed':
      return { tone: 'warn', text: failure(view.status, view.message) };
    case 'done':
      return done(view.result);
  }
}

function refusal(reason: 'signed_out' | 'session_network' | 'session_invalid'): string {
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

function done(result: SyncDocumentsResult): CloudSyncLine {
  const trouble = result.failed.length > 0 || result.stopped !== null || result.not_tried > 0;
  const parts = [counts(result)];
  if (result.failed.length > 0) parts.push(`${result.failed.length} failed`);
  if (result.stopped) {
    parts.push(`stopped early: ${result.stopped.error}`);
  } else if (result.failed.length > 0) {
    parts.push(`first: ${result.failed[0].error}`);
  }
  if (result.not_tried > 0) parts.push(`${result.not_tried} not tried, sync again to continue`);
  return { tone: trouble ? 'warn' : 'neutral', text: parts.join(' · ') };
}

function counts(result: SyncDocumentsResult): string {
  return `Uploaded ${result.uploaded} · Pulled ${result.pulled} · Unchanged ${result.skipped}`;
}
