import { createModuleStore } from '../../shared/module-store.ts';

/** Notes list and article share this width. Collapse must not write it. */
export const NOTES_SIDEBAR_WIDTH_KEY = 'cta_sidebar_width';
export const NOTES_SIDEBAR_DEFAULT_WIDTH = 221;
export const NOTES_SIDEBAR_MIN_WIDTH = 180;
export const NOTES_SIDEBAR_MAX_WIDTH = 400;

export const notesSidebarWidthStore = createModuleStore(NOTES_SIDEBAR_DEFAULT_WIDTH);

export function clampNotesSidebarWidth(px: number): number {
  return Math.min(NOTES_SIDEBAR_MAX_WIDTH, Math.max(NOTES_SIDEBAR_MIN_WIDTH, px));
}

/** Missing, invalid, or unreadable storage keeps the session width, or the default on first read. */
export function readNotesSidebarWidth(): number {
  try {
    const storage = globalThis.localStorage;
    if (!storage || typeof storage.getItem !== 'function') {
      return notesSidebarWidthStore.getSnapshot();
    }
    const saved = parseInt(storage.getItem(NOTES_SIDEBAR_WIDTH_KEY) ?? '', 10);
    if (!Number.isFinite(saved)) return NOTES_SIDEBAR_DEFAULT_WIDTH;
    return clampNotesSidebarWidth(saved);
  } catch {
    return notesSidebarWidthStore.getSnapshot();
  }
}

export function rememberNotesSidebarWidth(px: number): number {
  const width = clampNotesSidebarWidth(px);
  notesSidebarWidthStore.set(width);
  try {
    globalThis.localStorage?.setItem?.(NOTES_SIDEBAR_WIDTH_KEY, String(width));
  } catch {
    // The session already holds the width.
  }
  return width;
}
