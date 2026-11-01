import { createModuleStore } from '../../shared/module-store.ts';
import type { SyncDocumentsResult } from '../../host/api.ts';

/** Why the sync was not started: nothing was sent to the cloud. */
export type CloudSyncRefusal = 'signed_out' | 'session_network' | 'session_invalid';

/** What the account menu remembers about the last sync. */
export type CloudSyncView =
  | { phase: 'idle' }
  | { phase: 'syncing' }
  /** A sync was made. Part of it may still have gone wrong: see `result.failed` and `stopped`. */
  | { phase: 'done'; result: SyncDocumentsResult }
  | { phase: 'refused'; reason: CloudSyncRefusal }
  /** The command answered that no sync was made. `status` is its HTTP-like status, if it had one. */
  | { phase: 'failed'; status: number | null; message: string };

export const cloudSyncStore = createModuleStore<CloudSyncView>({ phase: 'idle' });
