import type { HubSession } from './store.ts';

/** How many chats the sidebar lists; the rest live in the All chats popover. */
export const SIDEBAR_CHAT_LIMIT = 3;

/**
 * The chats the sidebar lists: the newest `limit`. A current chat that is older than that takes
 * the last slot, so the chat you are in never drops out of the list.
 */
export function sidebarSessions(
  sessions: HubSession[],
  currentSessionId: string,
  limit: number = SIDEBAR_CHAT_LIMIT,
): HubSession[] {
  const recent = sessions.slice(0, limit);
  if (!currentSessionId || sessions.length <= limit) return recent;
  if (recent.some((s) => String(s.session_id || '') === currentSessionId)) return recent;
  const current = sessions.find((s) => String(s.session_id || '') === currentSessionId);
  return current ? [...recent.slice(0, limit - 1), current] : recent;
}
