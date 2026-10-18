import * as api from '../../host/api.ts';
import { state } from '../../host/state.ts';
import { enterProject, enterWorkspace } from '../../notes/commands/restore.ts';
import { showWorkspace, switchWorkspace } from '../../notes/commands/workspace-switch.ts';
import { setHomeState, type HomeProject } from '../state/store.ts';

function asProjects(data: unknown): HomeProject[] {
  const rec = data as {
    projects?: Array<{ project_id?: unknown; project_name?: unknown; icon?: unknown }>;
    error?: string;
  } | null;
  if (rec?.error) throw new Error(rec.error);
  const out: HomeProject[] = [];
  for (const row of rec?.projects ?? []) {
    const id = typeof row.project_id === 'string' ? row.project_id : '';
    if (!id) continue;
    out.push({
      id,
      title: typeof row.project_name === 'string' && row.project_name ? row.project_name : id,
      icon: typeof row.icon === 'string' ? row.icon : '',
    });
  }
  return out;
}

/** Load the enabled projects of every enabled workspace for the Home sidebar. */
export async function refreshWorkspaceProjects() {
  const ids = state.workspaces.list.map((w) => w.workspace_id);
  const loaded = await Promise.all(
    ids.map(async (id): Promise<[string, HomeProject[]]> => {
      try {
        return [id, asProjects(await api.fetchProjects(id))];
      } catch {
        return [id, []];
      }
    }),
  );
  setHomeState((prev) => ({ ...prev, workspaceProjects: Object.fromEntries(loaded) }));
}

/** Enter a workspace with its last selected project, also when it is already the shown one. */
export function openWorkspace(id: string, navigateFn?: (hash: string) => void) {
  const alreadyShown = id === state.workspaces.current;
  // A workspace that is switched to restores in `loadIndex`, once its index is there.
  if (alreadyShown) enterWorkspace();
  else switchWorkspace(id);
  navigateFn?.('#/spark');
}

/** Enter a workspace and select one of its projects once that workspace's index is loaded. */
export async function openProject(id: string, projectId: string, navigateFn?: (hash: string) => void) {
  await showWorkspace(id);
  enterProject(projectId);
  navigateFn?.('#/spark');
}
