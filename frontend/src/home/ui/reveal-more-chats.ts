import { useCallback, useEffect, type RefObject } from 'react';

/** Scrolling this close to the bottom of the chat list reveals the next page. */
const LOAD_MORE_PX = 40;

/**
 * Reveals the next page of chats when the sidebar chat list is scrolled near its bottom, and also
 * when the revealed chats do not fill the list (a tall window has no scrollbar to scroll). Both
 * are the same check, so the list is either scrollable or complete.
 *
 * Nothing is revealed while the list is not on screen (`clientHeight` is 0 when its pane is
 * hidden), otherwise a hidden list would reveal every chat. Revealing is a no-op once all chats
 * are shown, which ends the fill loop.
 *
 * Returns the handler for the list's `onScroll`.
 */
export function useRevealMoreChats(
  listRef: RefObject<HTMLElement | null>,
  revealMore: () => void,
  /** Re-check after these change: chats revealed, which pane is showing. */
  deps: readonly unknown[],
) {
  const check = useCallback(() => {
    const el = listRef.current;
    if (!el || el.clientHeight === 0) return;
    if (el.scrollHeight - el.scrollTop - el.clientHeight <= LOAD_MORE_PX) revealMore();
  }, [listRef, revealMore]);

  useEffect(() => {
    check();
    window.addEventListener('resize', check);
    return () => window.removeEventListener('resize', check);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- `deps` is the caller's trigger list
  }, [check, ...deps]);

  return check;
}
