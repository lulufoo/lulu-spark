import { readGet, writePost } from './transport.ts';

export type ViewMode = 'directory' | 'timeline';

export async function fetchWorkspaces() {
  return readGet('/api/workspaces?_=' + Date.now());
}

/** `icon` is an icon id; leave it out for the default icon, `''` also means none. */
export async function createWorkspace(name: string, viewMode: ViewMode = 'directory', icon?: string) {
  return writePost('/api/workspace-create', {
    name,
    view_mode: viewMode,
    ...(icon !== undefined ? { icon } : {}),
  });
}

export async function renameWorkspace(id: string, name: string) {
  return writePost('/api/workspace-rename', { id, name });
}

export async function setWorkspaceViewMode(id: string, viewMode: ViewMode) {
  return writePost('/api/workspace-view-mode', { id, view_mode: viewMode });
}

/** Set the workspace icon; `''` clears it (back to the default icon). */
export async function setWorkspaceIcon(id: string, icon: string) {
  return writePost('/api/workspace-icon', { id, icon });
}

export async function fetchDisabledWorkspaces() {
  return readGet('/api/workspaces-disabled?_=' + Date.now());
}

/** Every workspace id, enabled and disabled, in the order they should be listed. */
export async function reorderWorkspaces(ids: string[]) {
  return writePost('/api/workspaces-reorder', { ids });
}

/** Every project folder of the workspace, enabled and disabled, in the order to list them. */
export async function reorderProjects(workspace: string, folders: string[]) {
  return writePost('/api/projects-reorder', { workspace, folders });
}

export async function disableWorkspace(id: string) {
  return writePost('/api/workspace-disable', { id });
}

export async function enableWorkspace(id: string) {
  return writePost('/api/workspace-enable', { id });
}

export async function fetchProjects(workspace: string) {
  return readGet(`/api/projects?workspace=${encodeURIComponent(workspace)}&_=${Date.now()}`);
}

export async function createProject(workspace: string, title: string, description = '', icon?: string) {
  return writePost('/api/project-create', {
    workspace,
    title,
    description,
    ...(icon !== undefined ? { icon } : {}),
  });
}

/** `icon` left out keeps the project's icon; `''` clears it (back to the default icon). */
export async function updateProject(
  workspace: string,
  project: string,
  title: string,
  description = '',
  icon?: string,
) {
  return writePost('/api/project-update', {
    workspace,
    project,
    title,
    description,
    ...(icon !== undefined ? { icon } : {}),
  });
}

export async function fetchDisabledProjects(workspace: string) {
  return readGet(`/api/projects-disabled?workspace=${encodeURIComponent(workspace)}&_=${Date.now()}`);
}

export async function disableProject(workspace: string, project: string) {
  return writePost('/api/project-disable', { workspace, project });
}

export async function enableProject(workspace: string, project: string) {
  return writePost('/api/project-enable', { workspace, project });
}

export type HidePatternRow = { id: string; pattern: string };

/** Hidden files rules of one workspace. */
export async function fetchHidePatterns(workspace: string) {
  return readGet(`/api/hide-patterns?workspace=${encodeURIComponent(workspace)}&_=${Date.now()}`);
}

export async function addHidePattern(workspace: string, pattern: string) {
  return writePost('/api/hide-pattern-add', { workspace, pattern });
}

export async function updateHidePattern(workspace: string, id: string, pattern: string) {
  return writePost('/api/hide-pattern-update', { workspace, id, pattern });
}

export async function removeHidePattern(workspace: string, id: string) {
  return writePost('/api/hide-pattern-remove', { workspace, id });
}

/** The document a workspace's page had open (`{ id }`, empty when none). */
export async function fetchViewerState(workspace: string) {
  return readGet(`/api/viewer-state?workspace=${encodeURIComponent(workspace)}&_=${Date.now()}`);
}

export async function saveViewerState(workspace: string, id: string) {
  return writePost('/api/viewer-state', { workspace, id });
}
