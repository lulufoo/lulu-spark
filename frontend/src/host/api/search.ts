import { getReadDriver } from './transport.ts';

/** Header "Rebuild index": every workspace, index only (no git pull). */
export async function reindexAll() {
  const { invokeSearch } = await import('../apiClient.ts');
  return invokeSearch('reindexAll');
}

export async function getReindexAllStatus() {
  const { invokeSearch } = await import('../apiClient.ts');
  return invokeSearch('getReindexAllStatus');
}

/**
 * Search documents. By default one workspace (the Default workspace when none is given);
 * `allWorkspaces` searches every workspace instead and wins over `workspaceId`. Each hit carries
 * the `workspace_id` it lives in.
 */
export async function searchSpark(
  q: string,
  limit = 10,
  workspaceId = '',
  { allWorkspaces = false }: { allWorkspaces?: boolean } = {},
) {
  const params = new URLSearchParams({ q, limit: String(limit) });
  if (allWorkspaces) params.set('all_workspaces', '1');
  else if (workspaceId) params.set('workspace_id', workspaceId);
  const res = await getReadDriver().fetchGet(`/api/search-spark?${params.toString()}`);
  return res.json();
}

/** One chat found by title. */
export type ChatSearchHit = { session_id: string; title: string; updated_at: number };

/**
 * Search chats by title over the whole history, newest first (matching is done by the host).
 * Resolves to `{ hits }`.
 */
export async function searchChats(q: string, limit = 20): Promise<{ hits: ChatSearchHit[] }> {
  const params = new URLSearchParams({ q, limit: String(limit) });
  const res = await getReadDriver().fetchGet(`/api/search-chats?${params.toString()}`);
  return res.json();
}
