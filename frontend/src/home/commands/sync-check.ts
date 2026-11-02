import { refreshAuthSession } from '../../auth/refresh.ts';
import { checkSyncDocuments } from '../../host/api.ts';
import { isBusy } from '../state/sync-entry-flow.ts';
import { syncCheckStore, syncRunStore, type SyncCheck } from '../state/sync-entry.ts';

const NONE: SyncCheck = { phase: 'none' };

/** Which check is the latest: an answer that comes after a newer one was asked for is dropped. */
let latest = 0;

/**
 * Asks the host whether a sync would have anything to do, and puts the answer in the store. Call it
 * once the app has started and the user is signed in, once after each round of sync, and once more
 * after signing in.
 *
 * Nothing is asked of the cloud when nobody is signed in. A check that cannot be made (the sign-in
 * cannot be made fresh, the host or the cloud failed) is "nothing to sync": it shows no mark and
 * tells nothing, because the user did not ask for it and Start is always there to try again.
 *
 * Nothing is asked while a round is going on; the round asks again when it ends.
 */
export async function checkSyncPending(): Promise<void> {
  if (isBusy(syncRunStore.getSnapshot())) return;
  const mine = ++latest;
  syncCheckStore.set({ phase: 'checking' });
  const answer = await ask();
  if (mine === latest) syncCheckStore.set(answer);
}

/** A round begins: a check that is on its way is about the state before it, so its answer is dropped. */
export function dropPendingSyncCheck(): void {
  latest += 1;
  if (syncCheckStore.getSnapshot().phase === 'checking') syncCheckStore.set({ phase: 'unchecked' });
}

async function ask(): Promise<SyncCheck> {
  try {
    const session = await refreshAuthSession();
    if (session.kind !== 'ok') return NONE;
    const { uploads, pulls } = await checkSyncDocuments();
    return uploads + pulls > 0 ? { phase: 'pending', uploads, pulls } : NONE;
  } catch {
    return NONE;
  }
}
