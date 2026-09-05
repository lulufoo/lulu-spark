import { getHomeState } from '../state/store.ts';

/** Copy the current chat session id. Returns false when there is nothing to write. */
export async function copyCurrentSessionId() {
  const id = String(getHomeState().currentSessionId || '').trim();
  if (!id) return false;
  if (!navigator.clipboard?.writeText) return false;
  try {
    await navigator.clipboard.writeText(id);
    return true;
  } catch {
    return false;
  }
}
