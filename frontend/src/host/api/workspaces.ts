import { readGet, writePost } from './transport.ts';

export type ViewMode = 'directory' | 'timeline';

export async function fetchWorkspaces() {
  return readGet('/api/workspaces?_=' + Date.now());
}

/** `icon` is an icon id; leave it out for the default icon, `''` also means none. */
export async function createWorkspace(
  workspaceName: string,
  viewMode: ViewMode = 'directory',
  icon?: string,
  description?: string,
) {
  return writePost('/api/workspace-create', {
    workspace_name: workspaceName,
    view_mode: viewMode,
    ...(icon !== undefined ? { icon } : {}),
    ...(description !== undefined ? { description } : {}),
  });
}

export async function renameWorkspace(workspaceId: string, workspaceName: string) {
  return writePost('/api/workspace-rename', { workspace_id: workspaceId, workspace_name: workspaceName });
}

export async function setWorkspaceViewMode(workspaceId: string, viewMode: ViewMode) {
  return writePost('/api/workspace-view-mode', { workspace_id: workspaceId, view_mode: viewMode });
}

/** Set the workspace icon; `''` clears it (back to the default icon). */
export async function setWorkspaceIcon(workspaceId: string, icon: string) {
  return writePost('/api/workspace-icon', { workspace_id: workspaceId, icon });
}

/** Set the workspace description; `''` clears it. */
export async function setWorkspaceDescription(workspaceId: string, description: string) {
  return writePost('/api/workspace-description', { workspace_id: workspaceId, description });
}

export async function fetchDisabledWorkspaces() {
  return readGet('/api/workspaces-disabled?_=' + Date.now());
}

/** Every workspace id, enabled and disabled, in the order they should be listed. */
export async function reorderWorkspaces(ids: string[]) {
  return writePost('/api/workspaces-reorder', { ids });
}

/** Every project id of the workspace, in the order to list them. */
export async function reorderProjects(workspaceId: string, ids: string[]) {
  return writePost('/api/projects-reorder', { workspace_id: workspaceId, ids });
}

export async function disableWorkspace(workspaceId: string) {
  return writePost('/api/workspace-disable', { workspace_id: workspaceId });
}

export async function enableWorkspace(workspaceId: string) {
  return writePost('/api/workspace-enable', { workspace_id: workspaceId });
}

export async function fetchProjects(workspaceId: string) {
  return readGet(`/api/projects?workspace_id=${encodeURIComponent(workspaceId)}&_=${Date.now()}`);
}

export async function createProject(workspaceId: string, projectName: string, description = '', icon?: string) {
  return writePost('/api/project-create', {
    workspace_id: workspaceId,
    project_name: projectName,
    description,
    ...(icon !== undefined ? { icon } : {}),
  });
}

/** `icon` left out keeps the project's icon; `''` clears it (back to the default icon). */
export async function updateProject(
  projectId: string,
  projectName: string,
  description = '',
  icon?: string,
) {
  return writePost('/api/project-update', {
    project_id: projectId,
    project_name: projectName,
    description,
    ...(icon !== undefined ? { icon } : {}),
  });
}

export type DeleteProjectResult = {
  ok?: boolean;
  error?: string;
  project_id?: string;
  /** Documents given to the built-in project. */
  moved?: number;
  /** Of those, how many had to take a new file name. */
  renamed?: number;
  target_project_id?: string;
  target_project_name?: string;
};

/** Delete a project: its documents move to the workspace's built-in project first. */
export async function deleteProject(workspaceId: string, projectId: string) {
  return writePost('/api/project-delete', { workspace_id: workspaceId, project_id: projectId }) as Promise<DeleteProjectResult>;
}

export type HidePatternRow = { id: string; pattern: string };

/** Hidden files rules of one workspace. */
export async function fetchHidePatterns(workspaceId: string) {
  return readGet(`/api/hide-patterns?workspace_id=${encodeURIComponent(workspaceId)}&_=${Date.now()}`);
}

export async function addHidePattern(workspaceId: string, pattern: string) {
  return writePost('/api/hide-pattern-add', { workspace_id: workspaceId, pattern });
}

export async function updateHidePattern(workspaceId: string, id: string, pattern: string) {
  return writePost('/api/hide-pattern-update', { workspace_id: workspaceId, id, pattern });
}

export async function removeHidePattern(workspaceId: string, id: string) {
  return writePost('/api/hide-pattern-remove', { workspace_id: workspaceId, id });
}

/** What one project had selected: directory node + document (directory view) or day (timeline). */
export type ViewerProjectState = { node: string; doc: string; date: string };

/** The selection a workspace's page remembers, per project; `project_id` empty = All. */
export type ViewerState = { version: number; project_id: string; projects: Record<string, ViewerProjectState> };

/** What a workspace's page remembers; nothing remembered reads as empty. */
export async function fetchViewerState(workspaceId: string) {
  return readGet(`/api/viewer-state?workspace_id=${encodeURIComponent(workspaceId)}&_=${Date.now()}`);
}

/**
 * Record the last selected `project_id` ('' = All) and update that project's entry: a field left
 * out is kept, `''` clears it.
 */
export async function saveViewerState(
  workspaceId: string,
  patch: { project_id: string; node?: string; doc?: string; date?: string },
) {
  return writePost('/api/viewer-state', { workspace_id: workspaceId, ...patch });
}
