import { createModuleStore } from '../../shared/module-store.ts';
import type { SyncDocumentsResult } from '../../host/api.ts';

/** Why the sync was not started: nothing was sent to the cloud. */
export type CloudSyncRefusal = 'signed_out' | 'session_network' | 'session_invalid';

/** What the sync button remembers about the sync that is going on, or the last one. */
export type CloudSyncView =
  | { phase: 'idle' }
  /**
   * A sync is going on. `cancelling` is set once the user asked it to stop: it goes on until the
   * document being transferred is done.
   */
  | { phase: 'syncing'; cancelling: boolean }
  /** The user cancelled before the host was asked: nothing was sent or changed. */
  | { phase: 'cancelled' }
  /**
   * A sync was made. Part of it may still have gone wrong (see `result.failed` and `stopped`), or
   * the user may have cancelled it (`result.cancelled`).
   */
  | { phase: 'done'; result: SyncDocumentsResult }
  | { phase: 'refused'; reason: CloudSyncRefusal }
  /** The command answered that no sync was made. `status` is its HTTP-like status, if it had one. */
  | { phase: 'failed'; status: number | null; message: string };

export const cloudSyncStore = createModuleStore<CloudSyncView>({ phase: 'idle' });
