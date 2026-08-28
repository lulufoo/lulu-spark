// @ts-nocheck
import { createApiClient, resolveReadDriver } from '../../host/apiClient.ts';

function serviceError(data) {
  if (!data || typeof data !== 'object' || Array.isArray(data)) return null;
  if (!data.error) return null;
  const err = new Error(String(data.error));
  err.status = typeof data._status === 'number' ? data._status : 500;
  return err;
}

export async function loadAssistantTodoTasks() {
  const mode = resolveReadDriver();
  const client = createApiClient(resolveReadDriver(mode));
  const data = await client.getJson('/api/todo-tasks');
  const err = serviceError(data);
  if (err) throw err;
  return Array.isArray(data) ? data : [];
}

export function selectTop3ByCreatedAt(entries) {
  return [...entries]
    .sort((a, b) => {
      const aTime = Date.parse(a.created_at ?? '') || 0;
      const bTime = Date.parse(b.created_at ?? '') || 0;
      return bTime - aTime;
    })
    .slice(0, 3);
}

export function formatSubProgressSummary(master) {
  const subs = master.sub_tasks ?? [];
  const complete = subs.filter((sub) => sub.status === 'complete').length;
  const total = subs.length;
  return `${master.title} · ${complete}/${total} complete`;
}

export function buildDeepLink(masterId, subId) {
  const params = new URLSearchParams({ master: masterId, sub: subId });
  return `#/todo-tasks?${params.toString()}`;
}
