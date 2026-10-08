import { getPackageSnapshot } from '../../host/api.ts';
import { packageDebugStore } from '../state/package-debug.ts';

export async function loadPackageDebug() {
  try {
    const snap = await getPackageSnapshot();
    packageDebugStore.set(Boolean(snap.is_debug));
  } catch {
    packageDebugStore.set(false);
  }
}
