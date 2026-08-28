import { createApiClient, resolveReadDriver } from '../../host/apiClient.ts';
import type {
  StagedAttachment,
  TodoAttachment,
  TodoCategory,
  TodoComment,
  TodoMaster,
} from './types.ts';

/** Host category registry — built-in default id matches unspecified create fallback. */
export const DEFAULT_PLAN_CATEGORY_ID = 'uncategorized';

type TauriInvoke = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
type ServiceError = Error & { status?: number };

export function getTauriInvoke(): TauriInvoke | null {
  if (typeof window === 'undefined') return null;
  const invoke =
    window.__TAURI__?.core?.invoke || window.__TAURI_INTERNALS__?.invoke;
  return typeof invoke === 'function' ? invoke : null;
}

export function getTauriListen() {
  if (typeof window === 'undefined') return null;
  const listen = window.__TAURI__?.event?.listen;
  return typeof listen === 'function' ? listen : null;
}

function serviceError(data: unknown): ServiceError | null {
  if (!data || typeof data !== 'object' || Array.isArray(data)) return null;
  const rec = data as { error?: unknown; _status?: unknown };
  if (!rec.error) return null;
  const err = new Error(String(rec.error)) as ServiceError;
  err.status = typeof rec._status === 'number' ? rec._status : 500;
  return err;
}

export async function loadTodoTasks(): Promise<TodoMaster[]> {
  const mode = resolveReadDriver();
  const client = createApiClient(resolveReadDriver(mode));
  const data = await client.getJson('/api/todo-tasks');
  const err = serviceError(data);
  if (err) throw err;
  return Array.isArray(data) ? (data as TodoMaster[]) : [];
}

async function invokePlanWrite(command: string, args: Record<string, unknown>) {
  const invoke = getTauriInvoke();
  if (!invoke) {
    throw new Error('Tauri invoke unavailable');
  }
  const result = await invoke(command, args);
  const err = serviceError(result);
  if (err) throw err;
  return result;
}

export async function createTodoTask({
  title,
  subTitles,
}: { title?: string; subTitles?: string[] } = {}) {
  const args: Record<string, unknown> = { title };
  if (subTitles != null) {
    args.subTitles = subTitles;
  }
  const result = await invokePlanWrite('create_todo_task', args);
  return result && typeof result === 'object'
    ? (result as { master_task_id?: string; task?: { master_task_id?: string } })
    : {};
}

export async function deleteTodoTask({ masterTaskId }: { masterTaskId?: string } = {}) {
  return invokePlanWrite('delete_todo_task', { masterTaskId });
}

export async function addPlanSub({
  masterTaskId,
  title,
  content,
}: { masterTaskId?: string; title?: string; content?: string } = {}) {
  const args: Record<string, unknown> = { masterTaskId, title };
  if (content != null) {
    args.content = content;
  }
  return invokePlanWrite('add_todo_sub', args);
}

export async function deletePlanSub({
  masterTaskId,
  subTaskId,
}: { masterTaskId?: string; subTaskId?: string } = {}) {
  return invokePlanWrite('delete_todo_sub', { masterTaskId, subTaskId });
}

async function invokePlanPlain(command: string, args: Record<string, unknown>) {
  const invoke = getTauriInvoke();
  if (!invoke) {
    throw new Error('Tauri invoke unavailable');
  }
  const result = await invoke(command, args);
  const err = serviceError(result);
  if (err) throw err;
  return result;
}

export async function readPlanMd({ masterTaskId }: { masterTaskId?: string } = {}) {
  const result = await invokePlanPlain('read_todo_md', { masterTaskId });
  return typeof result === 'string' ? result : String(result ?? '');
}

export async function updatePlanMd({
  masterTaskId,
  planMd,
}: { masterTaskId?: string; planMd?: string } = {}) {
  await invokePlanPlain('update_todo_md', { masterTaskId, planMd });
}

export async function completePlan({
  masterTaskId,
  subTaskId,
}: { masterTaskId?: string; subTaskId?: string } = {}) {
  return invokePlanWrite('complete_todo', { masterTaskId, subTaskId });
}

export async function abandonPlanSub({
  masterTaskId,
  subTaskId,
}: { masterTaskId?: string; subTaskId?: string } = {}) {
  return invokePlanWrite('abandon_todo_sub', { masterTaskId, subTaskId });
}

export async function updatePlanSub({
  masterTaskId,
  subTaskId,
  title,
  content,
}: {
  masterTaskId?: string;
  subTaskId?: string;
  title?: string;
  content?: string;
} = {}) {
  const args: Record<string, unknown> = { masterTaskId, subTaskId, title };
  if (content != null) {
    args.content = content;
  }
  return invokePlanWrite('update_todo_sub', args);
}

export async function updatePlanMasterTitle({
  masterTaskId,
  title,
}: { masterTaskId?: string; title?: string } = {}) {
  return invokePlanWrite('update_todo_master_title', { masterTaskId, title });
}

export async function setPlanMasterStatus({
  masterTaskId,
  status,
}: { masterTaskId?: string; status?: string } = {}) {
  return invokePlanWrite('set_todo_master_status', { masterTaskId, status });
}

export async function listPlanCategories(): Promise<TodoCategory[]> {
  const result = await invokePlanPlain('list_todo_categories', {});
  if (Array.isArray(result)) return result as TodoCategory[];
  if (result && typeof result === 'object' && Array.isArray((result as { categories?: unknown }).categories)) {
    return (result as { categories: TodoCategory[] }).categories;
  }
  return [];
}

export async function createPlanCategory({ name }: { name?: string } = {}) {
  return invokePlanWrite('create_todo_category', { name });
}

export async function deletePlanCategory({ categoryId }: { categoryId?: string } = {}) {
  return invokePlanWrite('delete_todo_category', { categoryId });
}

/** Reassign todo category (Host set_todo_category → category_id update). */
export async function setPlanCategory({
  masterTaskId,
  categoryId,
}: { masterTaskId?: string; categoryId?: string } = {}) {
  return invokePlanWrite('set_todo_category', { masterTaskId, categoryId });
}

export async function listPlanAttachments({
  masterTaskId,
}: { masterTaskId?: string } = {}): Promise<TodoAttachment[]> {
  const result = await invokePlanPlain('list_todo_attachments', { masterTaskId });
  if (Array.isArray(result)) return result as TodoAttachment[];
  if (result && typeof result === 'object' && Array.isArray((result as { attachments?: unknown }).attachments)) {
    return (result as { attachments: TodoAttachment[] }).attachments;
  }
  return [];
}

/** Stage markdown into Host cache; returns `{ source_path }` for add/save. */
export async function stagePlanAttachmentSource({
  preferredName,
  content,
}: { preferredName?: string; content?: string } = {}): Promise<StagedAttachment> {
  const result = await invokePlanPlain('stage_todo_attachment_source', {
    preferredName,
    content,
  });
  return result && typeof result === 'object' ? (result as StagedAttachment) : {};
}

export async function addPlanAttachment({
  masterTaskId,
  sourcePath,
}: { masterTaskId?: string; sourcePath?: string } = {}) {
  return invokePlanPlain('add_todo_attachment', { masterTaskId, sourcePath });
}

export async function readPlanAttachment({
  masterTaskId,
  fileName,
}: { masterTaskId?: string; fileName?: string } = {}): Promise<string | { content?: string }> {
  const result = await invokePlanPlain('read_todo_attachment', { masterTaskId, fileName });
  if (typeof result === 'string') return result;
  if (result && typeof result === 'object') return result as { content?: string };
  return '';
}

export async function savePlanAttachment({
  masterTaskId,
  fileName,
  sourcePath,
}: { masterTaskId?: string; fileName?: string; sourcePath?: string } = {}) {
  return invokePlanPlain('save_todo_attachment', { masterTaskId, fileName, sourcePath });
}

export async function deletePlanAttachment({
  masterTaskId,
  fileName,
}: { masterTaskId?: string; fileName?: string } = {}) {
  return invokePlanPlain('delete_todo_attachment', { masterTaskId, fileName });
}

export async function listPlanComments({
  masterTaskId,
}: { masterTaskId?: string } = {}): Promise<TodoComment[]> {
  const result = await invokePlanPlain('list_todo_comments', { masterTaskId });
  if (Array.isArray(result)) return result as TodoComment[];
  if (result && typeof result === 'object' && Array.isArray((result as { comments?: unknown }).comments)) {
    return (result as { comments: TodoComment[] }).comments;
  }
  return [];
}

export async function addPlanComment({
  masterTaskId,
  body,
}: { masterTaskId?: string; body?: string } = {}) {
  return invokePlanPlain('add_todo_comment', { masterTaskId, body });
}

export async function updatePlanComment({
  masterTaskId,
  commentId,
  body,
}: { masterTaskId?: string; commentId?: string; body?: string } = {}) {
  return invokePlanPlain('update_todo_comment', { masterTaskId, commentId, body });
}

export async function deletePlanComment({
  masterTaskId,
  commentId,
}: { masterTaskId?: string; commentId?: string } = {}) {
  return invokePlanPlain('delete_todo_comment', { masterTaskId, commentId });
}

function basenameFromPath(path: string) {
  const normalized = String(path).replace(/\\/g, '/');
  const parts = normalized.split('/').filter(Boolean);
  return parts[parts.length - 1] || 'attachment.md';
}

/**
 * Host dialog: pick a local `.md`, then read content via convertFileSrc asset URL.
 * @returns {Promise<{ fileName: string, content: string } | null>} null when cancelled
 */
export async function pickLocalMarkdownFile() {
  if (typeof window === 'undefined') {
    throw new Error('File picker unavailable');
  }
  const dialog = window.__TAURI__?.dialog;
  if (typeof dialog?.open !== 'function') {
    throw new Error('File picker unavailable');
  }
  const selected = await dialog.open({
    multiple: false,
    filters: [{ name: 'Markdown', extensions: ['md'] }],
  });
  if (selected == null || selected === '') {
    return null;
  }
  const path = Array.isArray(selected) ? selected[0] : selected;
  if (!path) return null;

  const convertFileSrc = window.__TAURI__?.core?.convertFileSrc;
  const readAsset = typeof window.fetch === 'function' ? window.fetch.bind(window) : null;
  if (typeof convertFileSrc !== 'function' || !readAsset) {
    throw new Error('Could not read selected file');
  }
  const res = await readAsset(convertFileSrc(path));
  if (!res.ok) {
    throw new Error('Could not read selected file');
  }
  return { fileName: basenameFromPath(path), content: await res.text() };
}
