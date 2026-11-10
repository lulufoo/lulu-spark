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
