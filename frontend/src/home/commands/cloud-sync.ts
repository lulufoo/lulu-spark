import { refreshAuthSession } from '../../auth/refresh.ts';
import { syncDocuments } from '../../host/api.ts';
import { cloudSyncStore, type CloudSyncView } from '../state/cloud-sync.ts';

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
  cloudSyncStore.set({ phase: 'syncing' });
  cloudSyncStore.set(await outcome());
}

/** Forget the last sync, e.g. when the user signs out. */
export function resetCloudSync(): void {
  if (cloudSyncStore.getSnapshot().phase === 'syncing') return;
  cloudSyncStore.set({ phase: 'idle' });
}

async function outcome(): Promise<CloudSyncView> {
  try {
    const session = await refreshAuthSession();
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
