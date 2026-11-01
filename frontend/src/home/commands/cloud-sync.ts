import { refreshAuthSession } from '../../auth/refresh.ts';
import { cancelSyncDocuments, syncDocuments } from '../../host/api.ts';
import { cloudSyncStore, type CloudSyncView } from '../state/cloud-sync.ts';

/** How often a cancel is repeated while the host has not yet said that it took it. */
export const CANCEL_RETRY_MS = 250;
/**
 * How long a cancel is repeated. The sign-in refresh is cut at 10 s, so by then the host has been
 * asked to sync, and a cancel it answers is a cancel it took.
 */
export const CANCEL_GIVE_UP_MS = 30_000;

function cancelAsked(): boolean {
  const view = cloudSyncStore.getSnapshot();
  return view.phase === 'syncing' && view.cancelling;
}

/**
 * One sync on the user's request: make the sign-in session fresh, then ask the host to sync.
 *
 * The host reads the session from the vault and does not refresh it, so the refresh happens here,
 * first. When it cannot be done the host is not asked, and the store says why.
 *
 * A second request while one is going on does nothing. There is no retry: the user asks again,
 * and the host plans from the versions as they are by then.
 */
export async function runCloudSync(): Promise<void> {
  if (cloudSyncStore.getSnapshot().phase === 'syncing') return;
  cloudSyncStore.set({ phase: 'syncing', cancelling: false });
  cloudSyncStore.set(await outcome());
}

/**
 * The user asks the sync that is going on to stop. Nothing happens when none is.
 *
 * Before the host has a sync going (the sign-in is being refreshed, or the request is on its way)
 * it answers that there is nothing to cancel, and remembers nothing. So the cancel is asked again
 * until the host takes it, or the sync is over; and `runCloudSync` does not ask the host to sync
 * at all when the user cancelled while the sign-in was being refreshed.
 *
 * The host stops before the next document: the one being transferred is finished, and the sync
 * then answers with `cancelled: true`.
 */
export async function cancelCloudSync(): Promise<void> {
  const view = cloudSyncStore.getSnapshot();
  if (view.phase !== 'syncing' || view.cancelling) return;
  cloudSyncStore.set({ phase: 'syncing', cancelling: true });
  const giveUpAt = Date.now() + CANCEL_GIVE_UP_MS;
  while (cancelAsked() && Date.now() < giveUpAt) {
    try {
      if ((await cancelSyncDocuments()).cancelling) return;
    } catch {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, CANCEL_RETRY_MS));
  }
}

/** Forget the last sync, e.g. when the user signs out. */
export function resetCloudSync(): void {
  if (cloudSyncStore.getSnapshot().phase === 'syncing') return;
  cloudSyncStore.set({ phase: 'idle' });
}

async function outcome(): Promise<CloudSyncView> {
  try {
    const session = await refreshAuthSession();
    if (cancelAsked()) return { phase: 'cancelled' };
    if (session.kind === 'signed_out') return { phase: 'refused', reason: 'signed_out' };
    if (session.kind === 'failed') {
      return { phase: 'refused', reason: session.reason === 'network' ? 'session_network' : 'session_invalid' };
    }
    const result = await syncDocuments();
    if (result.pulled > 0) document.dispatchEvent(new CustomEvent('cta:reload'));
    return { phase: 'done', result };
  } catch (error) {
    const status = (error as { status?: unknown } | null)?.status;
    return {
      phase: 'failed',
      status: typeof status === 'number' ? status : null,
      message: error instanceof Error && error.message ? error.message : 'Sync failed',
    };
  }
}
