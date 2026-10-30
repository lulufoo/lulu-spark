import { fetchProjects } from '../../../host/api.ts';

function countLabel(count: number, one: string, many: string): string {
  return count === 1 ? `1 ${one}` : `${count} ${many}`;
}

/** What the confirm dialog tells the user will happen. */
export function describeWorkspaceDelete(
  name: string,
  docCount: number,
  projectCount: number,
  defaultWorkspaceName: string,
  inboxName: string,
): string {
  const projects = countLabel(projectCount, 'project', 'projects');
  if (docCount <= 0) {
    return `“${name}” has no documents. It will be deleted with its ${projects}. This cannot be undone.`;
  }
  const docs = countLabel(docCount, 'document', 'documents');
  return `${docs} in “${name}” will be moved to “${inboxName}” in “${defaultWorkspaceName}”, then “${name}” and its ${projects} are deleted. This cannot be undone.`;
}

export type WorkspaceDeleteCounts = {
  docCount: number;
  projectCount: number;
  inboxName: string;
};

/**
 * Document and project counts for the delete dialog. Fresh each time; a failed
 * fetch throws so the dialog never opens with a fake zero.
 */
export async function summarizeWorkspaceDeleteCounts(workspaceId: string): Promise<WorkspaceDeleteCounts> {
  const data = (await fetchProjects(workspaceId)) as {
    projects?: Array<{ project_id?: unknown; project_name?: unknown; is_default?: unknown; doc_count?: unknown }>;
    error?: string;
  };
  if (data?.error) throw new Error(data.error);
  const projects = data?.projects;
  if (!Array.isArray(projects)) throw new Error('Could not load projects');
  let docCount = 0;
  let inboxName = '';
  for (const project of projects) {
    if (typeof project.doc_count === 'number') docCount += project.doc_count;
    if (project.is_default === true) {
      inboxName = typeof project.project_name === 'string' && project.project_name ? project.project_name : String(project.project_id ?? '');
    }
  }
  if (!inboxName) throw new Error('Inbox missing');
  return { docCount, projectCount: projects.length, inboxName };
}
