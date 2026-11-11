import { createModuleStore } from '../../shared/module-store.ts';

/** Home chat sidebar width. Collapse must not write it. */
export const HOME_SIDEBAR_WIDTH_KEY = 'home_chat_sidebar_width';
export const HOME_SIDEBAR_DEFAULT_WIDTH = 248;
export const HOME_SIDEBAR_MIN_WIDTH = 180;
export const HOME_SIDEBAR_MAX_WIDTH = 480;

export const homeSidebarWidthStore = createModuleStore(HOME_SIDEBAR_DEFAULT_WIDTH);

export function clampHomeSidebarWidth(px: number): number {
  return Math.min(HOME_SIDEBAR_MAX_WIDTH, Math.max(HOME_SIDEBAR_MIN_WIDTH, px));
}

/** Missing or invalid storage uses the default. Unreadable storage keeps the session width. */
export function readHomeSidebarWidth(): number {
  try {
    const storage = globalThis.localStorage;
    if (!storage || typeof storage.getItem !== 'function') {
      return homeSidebarWidthStore.getSnapshot();
    }
    const saved = parseInt(storage.getItem(HOME_SIDEBAR_WIDTH_KEY) ?? '', 10);
    if (!Number.isFinite(saved)) return HOME_SIDEBAR_DEFAULT_WIDTH;
    return clampHomeSidebarWidth(saved);
  } catch {
    return homeSidebarWidthStore.getSnapshot();
  }
}

export function rememberHomeSidebarWidth(px: number): number {
  const width = clampHomeSidebarWidth(px);
  homeSidebarWidthStore.set(width);
  try {
    globalThis.localStorage?.setItem?.(HOME_SIDEBAR_WIDTH_KEY, String(width));
  } catch {
    // The session already holds the width.
  }
  return width;
}
