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

/** Search the documents of one workspace (the Default workspace when none is given). */
export async function searchSpark(q: string, limit = 10, workspaceId = '') {
  const params = new URLSearchParams({ q, limit: String(limit) });
  if (workspaceId) params.set('workspace_id', workspaceId);
  const res = await getReadDriver().fetchGet(`/api/search-spark?${params.toString()}`);
  return res.json();
}
