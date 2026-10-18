/**
 * Recent searches, shared by every page that searches documents. One localStorage list, newest
 * first, at most 10 entries; a leading `#` is not part of a query.
 */
export const SEARCH_HISTORY_KEY = 'gs-history-wb';
export const SEARCH_HISTORY_MAX = 10;

export function normalizeSearchQuery(raw: unknown): string {
  const trimmed = String(raw || '').trim();
  return trimmed.startsWith('#') ? trimmed.slice(1).trim() : trimmed;
}

export function getSearchHistory(): string[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(SEARCH_HISTORY_KEY) || '');
    return Array.isArray(parsed) ? parsed.filter((q): q is string => typeof q === 'string') : [];
  } catch {
    return [];
  }
}

export function addSearchHistory(q: string): void {
  const normalized = normalizeSearchQuery(q);
  if (!normalized) return;
  const list = getSearchHistory().filter((x) => x !== normalized);
  list.unshift(normalized);
  localStorage.setItem(SEARCH_HISTORY_KEY, JSON.stringify(list.slice(0, SEARCH_HISTORY_MAX)));
}

export function removeSearchHistory(q: string): void {
  const list = getSearchHistory().filter((x) => x !== q);
  localStorage.setItem(SEARCH_HISTORY_KEY, JSON.stringify(list));
}
