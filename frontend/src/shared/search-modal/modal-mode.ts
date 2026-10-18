import { searchChats, searchSpark } from '../../host/api.ts';
import { formatDayHeading } from '../day-heading.ts';
import { CHAT_SEARCH_HISTORY_KEY, SEARCH_HISTORY_KEY } from './history.ts';
import { hitLocation, type SearchHit, type SearchNames, type SearchScope } from './hit-location.ts';

/** What the modal searches: documents (workspace scope) or chats (by title, whole history). */
export type SearchMode = 'docs' | 'chats';

/** One row of the result list, whatever was searched. */
export type ResultRow = { id: string; title: string; sub: string };

/** `status` is the line to show when the search gave no rows because it failed. */
export type ModeSearchResult = { rows: ResultRow[]; status?: string };

export function getHistoryKey(mode: SearchMode): string {
  return mode === 'chats' ? CHAT_SEARCH_HISTORY_KEY : SEARCH_HISTORY_KEY;
}

export function getModeTexts(mode: SearchMode, scope: SearchScope) {
  if (mode === 'chats') {
    return {
      label: 'Search chats',
      placeholder: 'Search chats by title…',
      emptyHistory: 'Type to search chats.',
      noResults: 'No matching chats',
    };
  }
  return {
    label: 'Search documents',
    placeholder: scope.kind === 'all-workspaces' ? 'Search all workspaces…' : 'Search this workspace…',
    emptyHistory: 'Type to search documents.',
    noResults: 'No related results',
  };
}

/** Run one search. A thrown error is the caller's to show; a reported one comes back as `status`. */
export async function searchByMode(
  mode: SearchMode,
  q: string,
  limit: number,
  scope: SearchScope,
  names: SearchNames,
): Promise<ModeSearchResult> {
  if (mode === 'chats') {
    const data = await searchChats(q, limit);
    const now = new Date();
    const rows = (data.hits || []).map((hit) => ({
      id: hit.session_id,
      title: hit.title,
      sub: formatDayHeading(new Date(hit.updated_at * 1000), now),
    }));
    return { rows };
  }
  const data = await searchSpark(
    q,
    limit,
    scope.kind === 'workspace' ? scope.workspaceId : '',
    { allWorkspaces: scope.kind === 'all-workspaces' },
  );
  if (data.error === 'not_indexed') return { rows: [], status: 'Index not built yet.' };
  if (data.error) return { rows: [], status: 'Search error' };
  const rows = ((data.hits || []) as SearchHit[]).map((hit) => ({
    id: hit.id || '',
    title: hit.title || hit.id || '',
    sub: hitLocation(hit, scope, names) || '',
  }));
  return { rows };
}
