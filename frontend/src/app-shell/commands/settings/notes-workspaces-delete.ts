import * as api from '../../../host/api.ts';
import { announceRegistryChange } from '../../../shared/registry-events.ts';
import { state } from '../../../host/state.ts';
import { loadWorkspaces } from '../../../notes/commands/workspace.ts';
import { loadDisabledWorkspaces } from './workspaces.ts';
import { patchWorkspacesPanel, workspacesPanelStore } from '../../state/settings/workspaces-panel.ts';

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
  const data = (await api.fetchProjects(workspaceId)) as {
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

function describeError(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function clearDeleteFields() {
  return {
    deleteId: '',
    deleteError: '',
    deleteDocCount: 0,
    deleteProjectCount: 0,
    deleteInboxName: '',
  };
}

/** Open the confirm dialog after loading counts. The built-in workspace cannot be deleted. */
export async function askDeleteWorkspace(id: string) {
  const workspace = state.workspaces.list.find((w) => w.workspace_id === id);
  const snap = workspacesPanelStore.getSnapshot();
  if (!workspace || workspace.is_default || snap.busy) return;
  patchWorkspacesPanel({ error: '', deleteError: '', busy: true });
  try {
    const counts = await summarizeWorkspaceDeleteCounts(id);
    patchWorkspacesPanel({
      deleteId: id,
      deleteDocCount: counts.docCount,
      deleteProjectCount: counts.projectCount,
      deleteInboxName: counts.inboxName,
      busy: false,
    });
  } catch (err) {
    patchWorkspacesPanel({ busy: false, error: describeError(err), ...clearDeleteFields() });
  }
}

export function cancelDeleteWorkspace() {
  const snap = workspacesPanelStore.getSnapshot();
  if (snap.busy || !snap.deleteId) return;
  patchWorkspacesPanel(clearDeleteFields());
}

/**
 * Delete the workspace the dialog asks about. A repeat click while busy does nothing.
 * A failure stays in the dialog, refreshes the counts, and leaves the list as it is.
 */
export async function confirmDeleteWorkspace(): Promise<boolean> {
  const snap = workspacesPanelStore.getSnapshot();
  const id = snap.deleteId;
  if (!id || snap.busy) return false;
  const previous = state.workspaces.current;
  patchWorkspacesPanel({ busy: true, deleteError: '' });
  try {
    const result = await api.deleteWorkspace(id);
    if (result?.error) throw new Error(result.error);
  } catch (err) {
    let counts = {
      deleteDocCount: snap.deleteDocCount,
      deleteProjectCount: snap.deleteProjectCount,
      deleteInboxName: snap.deleteInboxName,
    };
    try {
      const fresh = await summarizeWorkspaceDeleteCounts(id);
      counts = {
        deleteDocCount: fresh.docCount,
        deleteProjectCount: fresh.projectCount,
        deleteInboxName: fresh.inboxName,
      };
    } catch {
      // Keep the counts the dialog already shows.
    }
    patchWorkspacesPanel({ busy: false, deleteError: describeError(err), ...counts });
    return false;
  }
  patchWorkspacesPanel({ busy: false, ...clearDeleteFields() });
  await loadWorkspaces();
  if (state.workspaces.current !== previous) document.dispatchEvent(new CustomEvent('cta:reload'));
  announceRegistryChange();
  await loadDisabledWorkspaces();
  return true;
}
