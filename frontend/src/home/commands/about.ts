import { getPackageSnapshot } from '../../host/api.ts';
import { aboutStore } from '../state/about.ts';

export async function openAbout() {
  try {
    const snap = await getPackageSnapshot();
    aboutStore.set({
      open: true,
      product_name: snap.product_name,
      version: snap.version,
    });
  } catch {
    aboutStore.set({
      open: true,
      product_name: '',
      version: '',
    });
  }
}

export function closeAbout() {
  aboutStore.set((prev) => ({ ...prev, open: false }));
}
