import { createApiClient, resolveReadDriver } from '../../host/apiClient.ts';
import type { TodoMaster, TodoSub } from '../state/types.ts';

type ServiceError = Error & { status?: number };

function serviceError(data: unknown): ServiceError | null {
  if (!data || typeof data !== 'object' || Array.isArray(data)) return null;
  const rec = data as { error?: unknown; _status?: unknown };
  if (!rec.error) return null;
  const err = new Error(String(rec.error)) as ServiceError;
  err.status = typeof rec._status === 'number' ? rec._status : 500;
  return err;
}

export async function loadAssistantTodoTasks(): Promise<TodoMaster[]> {
  const mode = resolveReadDriver();
  const client = createApiClient(resolveReadDriver(mode));
  const data = await client.getJson('/api/todo-tasks');
  const err = serviceError(data);
  if (err) throw err;
  return Array.isArray(data) ? (data as TodoMaster[]) : [];
}

export function selectTop3ByCreatedAt(entries: TodoMaster[]) {
  return [...entries]
    .sort((a, b) => {
      const aTime = Date.parse(String(a.created_at ?? '')) || 0;
      const bTime = Date.parse(String(b.created_at ?? '')) || 0;
      return bTime - aTime;
    })
    .slice(0, 3);
}

export function formatSubProgressSummary(master: TodoMaster) {
  const subs = master.sub_tasks ?? [];
  const complete = subs.filter((sub: TodoSub) => sub.status === 'complete').length;
  const total = subs.length;
  return `${master.title} · ${complete}/${total} complete`;
}

export function buildDeepLink(masterId: string, subId: string) {
  const params = new URLSearchParams({ master: masterId, sub: subId });
  return `#/todo-tasks?${params.toString()}`;
}
