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
