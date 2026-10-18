import type { SearchNames } from '../../shared/search-modal/hit-location.ts';
import type { HomeProject } from './store.ts';

/**
 * Names for the search results' location line, from what the Home sidebar has already loaded.
 * A workspace or project that is not loaded (for example a disabled project) gives '' and the
 * result row leaves that part out.
 */
export function homeSearchNames(
  workspaces: { workspace_id: string; workspace_name?: string }[],
  projectsByWorkspace: Record<string, HomeProject[]>,
): SearchNames {
  return {
    workspace: (id) => workspaces.find((w) => w.workspace_id === id)?.workspace_name || '',
    project: (workspaceId, projectId) =>
      projectsByWorkspace[workspaceId]?.find((p) => p.id === projectId)?.title || '',
  };
}
