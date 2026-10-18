/** What a search covers: every workspace (Home) or one workspace (the notes page). */
export type SearchScope = { kind: 'all-workspaces' } | { kind: 'workspace'; workspaceId: string };

export type SearchHit = {
  id?: string;
  title?: string;
  workspace_id?: string;
  project_id?: string;
};

/** Turns the ids a hit carries into the names to show. An unknown id gives ''. */
export type SearchNames = {
  workspace: (workspaceId: string) => string;
  project: (workspaceId: string, projectId: string) => string;
};

/**
 * The line under a result's title: where the document lives. `workspace / project` when the search
 * spans workspaces, just `project` inside one. A part whose name is not known is left out rather
 * than shown as a raw id.
 */
export function hitLocation(hit: SearchHit, scope: SearchScope, names: SearchNames): string {
  const workspaceId = String(hit.workspace_id || (scope.kind === 'workspace' ? scope.workspaceId : ''));
  const project = names.project(workspaceId, String(hit.project_id || ''));
  if (scope.kind === 'workspace') return project;
  return [names.workspace(workspaceId), project].filter(Boolean).join(' / ');
}
