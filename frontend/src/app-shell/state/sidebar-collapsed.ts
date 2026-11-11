import { createModuleStore } from '../../shared/module-store.ts';

/** Shared across Home, the list, and the article. Width prefs stay on their own keys. */
export const SIDEBAR_COLLAPSED_KEY = 'sidebar_collapsed';

/** `false` is expanded. That is the first-run value, before a preference exists. */
export const sidebarCollapsedStore = createModuleStore(false);

/**
 * Read the saved flag.
 * Missing or any value other than `true` / `false` means expanded.
 * When storage cannot be read, keep the in-session value so a toggle still works.
 */
export function recallSidebarCollapsed(): boolean {
  try {
    const storage = globalThis.localStorage;
    if (!storage || typeof storage.getItem !== 'function') {
      return sidebarCollapsedStore.getSnapshot();
    }
    const raw = storage.getItem(SIDEBAR_COLLAPSED_KEY);
    if (raw === 'true') return true;
    if (raw === 'false') return false;
    return false;
  } catch {
    return sidebarCollapsedStore.getSnapshot();
  }
}

/** Remember the flag for this session, then try to save it. A failed write leaves the session as-is. */
export function rememberSidebarCollapsed(collapsed: boolean): void {
  sidebarCollapsedStore.set(collapsed);
  try {
    globalThis.localStorage?.setItem?.(SIDEBAR_COLLAPSED_KEY, collapsed ? 'true' : 'false');
  } catch {
    // The session already holds the value.
  }
}
