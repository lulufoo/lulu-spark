/**
 * Recent searches, shared by every page that searches. One localStorage list per kind of search
 * (documents, chats), newest first, at most 10 entries; a leading `#` is not part of a query.
 */
export const SEARCH_HISTORY_KEY = 'gs-history-wb';
export const CHAT_SEARCH_HISTORY_KEY = 'gs-history-chats';
export const SEARCH_HISTORY_MAX = 10;

export function normalizeSearchQuery(raw: unknown): string {
  const trimmed = String(raw || '').trim();
  return trimmed.startsWith('#') ? trimmed.slice(1).trim() : trimmed;
}

export function getSearchHistory(key: string = SEARCH_HISTORY_KEY): string[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(key) || '');
    return Array.isArray(parsed) ? parsed.filter((q): q is string => typeof q === 'string') : [];
  } catch {
    return [];
  }
}

export function addSearchHistory(q: string, key: string = SEARCH_HISTORY_KEY): void {
  const normalized = normalizeSearchQuery(q);
  if (!normalized) return;
  const list = getSearchHistory(key).filter((x) => x !== normalized);
  list.unshift(normalized);
  localStorage.setItem(key, JSON.stringify(list.slice(0, SEARCH_HISTORY_MAX)));
}

export function removeSearchHistory(q: string, key: string = SEARCH_HISTORY_KEY): void {
  const list = getSearchHistory(key).filter((x) => x !== q);
  localStorage.setItem(key, JSON.stringify(list));
}
