import { createApiClient, resolveReadDriver } from '../host/apiClient.js';

/** Host category registry — built-in default id matches unspecified create fallback. */
export const DEFAULT_PLAN_CATEGORY_ID = 'uncategorized';

export function getTauriInvoke() {
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

function serviceError(data) {
  if (!data || typeof data !== 'object' || Array.isArray(data)) return null;
  if (!data.error) return null;
  const err = new Error(String(data.error));
  err.status = typeof data._status === 'number' ? data._status : 500;
  return err;
}

export async function loadTodoTasks() {
  const mode = resolveReadDriver();
  const client = createApiClient(resolveReadDriver(mode));
  const data = await client.getJson('/api/todo-tasks');
  const err = serviceError(data);
  if (err) throw err;
  return Array.isArray(data) ? data : [];
}

async function invokePlanWrite(command, args) {
  const invoke = getTauriInvoke();
  if (!invoke) {
    throw new Error('Tauri invoke unavailable');
  }
  const result = await invoke(command, args);
  const err = serviceError(result);
  if (err) throw err;
  return result;
}

export async function createTodoTask({ title, subTitles } = {}) {
  const args = { title };
  if (subTitles != null) {
    args.subTitles = subTitles;
  }
  return invokePlanWrite('create_todo_task', args);
}

export async function deleteTodoTask({ masterTaskId } = {}) {
  return invokePlanWrite('delete_todo_task', { masterTaskId });
}

export async function addPlanSub({ masterTaskId, title, content } = {}) {
  const args = { masterTaskId, title };
  if (content != null) {
    args.content = content;
  }
  return invokePlanWrite('add_todo_sub', args);
}

export async function deletePlanSub({ masterTaskId, subTaskId } = {}) {
  return invokePlanWrite('delete_todo_sub', { masterTaskId, subTaskId });
}

async function invokePlanPlain(command, args) {
  const invoke = getTauriInvoke();
  if (!invoke) {
    throw new Error('Tauri invoke unavailable');
  }
  const result = await invoke(command, args);
  const err = serviceError(result);
  if (err) throw err;
  return result;
}

export async function readPlanMd({ masterTaskId } = {}) {
  return invokePlanPlain('read_todo_md', { masterTaskId });
}

export async function updatePlanMd({ masterTaskId, planMd } = {}) {
  await invokePlanPlain('update_todo_md', { masterTaskId, planMd });
}

export async function completePlan({ masterTaskId, subTaskId } = {}) {
  return invokePlanWrite('complete_todo', { masterTaskId, subTaskId });
}

export async function abandonPlanSub({ masterTaskId, subTaskId } = {}) {
  return invokePlanWrite('abandon_todo_sub', { masterTaskId, subTaskId });
}

export async function updatePlanSub({ masterTaskId, subTaskId, title, content } = {}) {
  const args = { masterTaskId, subTaskId, title };
  if (content != null) {
    args.content = content;
  }
  return invokePlanWrite('update_todo_sub', args);
}

export async function updatePlanMasterTitle({ masterTaskId, title } = {}) {
  return invokePlanWrite('update_todo_master_title', { masterTaskId, title });
}

export async function setPlanMasterStatus({ masterTaskId, status } = {}) {
  return invokePlanWrite('set_todo_master_status', { masterTaskId, status });
}

export async function listPlanCategories() {
  const result = await invokePlanPlain('list_todo_categories', {});
  if (Array.isArray(result)) return result;
  if (result && Array.isArray(result.categories)) return result.categories;
  return [];
}

export async function createPlanCategory({ name } = {}) {
  return invokePlanWrite('create_todo_category', { name });
}

export async function deletePlanCategory({ categoryId } = {}) {
  return invokePlanWrite('delete_todo_category', { categoryId });
}

/** Reassign todo category (Host set_todo_category → category_id update). */
export async function setPlanCategory({ masterTaskId, categoryId } = {}) {
  return invokePlanWrite('set_todo_category', { masterTaskId, categoryId });
}

export async function listPlanAttachments({ masterTaskId } = {}) {
  const result = await invokePlanPlain('list_todo_attachments', { masterTaskId });
  if (Array.isArray(result)) return result;
  if (result && Array.isArray(result.attachments)) return result.attachments;
  return [];
}

/** Stage markdown into Host cache; returns `{ source_path }` for add/save. */
export async function stagePlanAttachmentSource({ preferredName, content } = {}) {
  return invokePlanPlain('stage_todo_attachment_source', { preferredName, content });
}

export async function addPlanAttachment({ masterTaskId, sourcePath } = {}) {
  return invokePlanPlain('add_todo_attachment', { masterTaskId, sourcePath });
}

export async function readPlanAttachment({ masterTaskId, fileName } = {}) {
  return invokePlanPlain('read_todo_attachment', { masterTaskId, fileName });
}

export async function savePlanAttachment({ masterTaskId, fileName, sourcePath } = {}) {
  return invokePlanPlain('save_todo_attachment', { masterTaskId, fileName, sourcePath });
}

export async function deletePlanAttachment({ masterTaskId, fileName } = {}) {
  return invokePlanPlain('delete_todo_attachment', { masterTaskId, fileName });
}

export async function listPlanComments({ masterTaskId } = {}) {
  const result = await invokePlanPlain('list_todo_comments', { masterTaskId });
  if (Array.isArray(result)) return result;
  if (result && Array.isArray(result.comments)) return result.comments;
  return [];
}

export async function addPlanComment({ masterTaskId, body } = {}) {
  return invokePlanPlain('add_todo_comment', { masterTaskId, body });
}

export async function updatePlanComment({ masterTaskId, commentId, body } = {}) {
  return invokePlanPlain('update_todo_comment', { masterTaskId, commentId, body });
}

export async function deletePlanComment({ masterTaskId, commentId } = {}) {
  return invokePlanPlain('delete_todo_comment', { masterTaskId, commentId });
}

function basenameFromPath(path) {
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
  const readAsset = typeof window['fetch'] === 'function' ? window['fetch'].bind(window) : null;
  if (typeof convertFileSrc !== 'function' || !readAsset) {
    throw new Error('Could not read selected file');
  }
  const res = await readAsset(convertFileSrc(path));
  if (!res.ok) {
    throw new Error('Could not read selected file');
  }
  return { fileName: basenameFromPath(path), content: await res.text() };
}
