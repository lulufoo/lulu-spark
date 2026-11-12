import { writePost } from './transport.ts';

/** Create one empty folder. `parent` is omitted for a folder at the project root. */
export async function createFolder(payload: {
  workspace_id: string;
  project_id: string;
  name: string;
  parent?: string;
}) {
  return writePost('/api/create-folder', payload);
}

/** Delete one empty folder. `path` is relative to the project, such as `plans/q1`. */
export async function deleteFolder(payload: {
  workspace_id: string;
  project_id: string;
  path: string;
}) {
  return writePost('/api/delete-folder', payload);
}

/** Rename one folder. `name` is the new single segment under the same parent. */
export async function renameFolder(payload: {
  workspace_id: string;
  project_id: string;
  path: string;
  name: string;
}) {
  return writePost('/api/rename-folder', payload);
}

/** Rename one document's markdown file in its folder. `name` is the new basename. */
export async function renameDoc(payload: {
  workspace_id: string;
  project_id: string;
  id: string;
  name: string;
}) {
  return writePost('/api/rename-doc', payload);
}

/** Move one document into another folder of the same project. `target` is relative to the project; omitted or '' for the root. */
export async function moveDocDir(payload: {
  workspace_id: string;
  project_id: string;
  id: string;
  target?: string;
}) {
  return writePost('/api/move-doc-dir', payload);
}
