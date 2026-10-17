import * as api from '../../host/api.ts';
import { state } from '../../host/state.ts';
import { selectDefaultProject, selectProject } from '../../notes/commands/sidebar.ts';
import { restoreOpenDoc } from '../../notes/commands/viewer-state.ts';
import { showWorkspace, switchWorkspace } from '../../notes/commands/workspace-switch.ts';
import { setHomeState, type HomeProject } from '../state/store.ts';

function asProjects(data: unknown): HomeProject[] {
  const rec = data as { projects?: Array<{ folder?: unknown; title?: unknown }>; error?: string } | null;
  if (rec?.error) throw new Error(rec.error);
  const out: HomeProject[] = [];
  for (const row of rec?.projects ?? []) {
    const folder = typeof row.folder === 'string' ? row.folder : '';
    if (!folder) continue;
    out.push({ folder, title: typeof row.title === 'string' && row.title ? row.title : folder });
  }
  return out;
}

/** Load the enabled projects of every enabled workspace for the Home sidebar. */
export async function refreshWorkspaceProjects() {
  const ids = state.workspaces.list.map((w) => w.id);
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

/** Enter a workspace with its default project, also when it is already the shown one. */
export function openWorkspace(id: string, navigateFn?: (hash: string) => void) {
  const alreadyShown = id === state.workspaces.current;
  if (alreadyShown) selectDefaultProject();
  else switchWorkspace(id);
  navigateFn?.('#/spark');
  // A workspace that is switched to restores in `loadIndex`, once its index is there.
  if (alreadyShown) void restoreOpenDoc();
}

/** Enter a workspace and select one of its projects once that workspace's index is loaded. */
export async function openProject(id: string, folder: string, navigateFn?: (hash: string) => void) {
  await showWorkspace(id);
  selectProject(folder);
  navigateFn?.('#/spark');
}
