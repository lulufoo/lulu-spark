import { readGet, writePost } from './transport.ts';

export type ViewMode = 'directory' | 'timeline';

export async function fetchWorkspaces() {
  return readGet('/api/workspaces?_=' + Date.now());
}

export async function createWorkspace(name: string, viewMode: ViewMode = 'directory') {
  return writePost('/api/workspace-create', { name, view_mode: viewMode });
}

export async function renameWorkspace(id: string, name: string) {
  return writePost('/api/workspace-rename', { id, name });
}

export async function setWorkspaceViewMode(id: string, viewMode: ViewMode) {
  return writePost('/api/workspace-view-mode', { id, view_mode: viewMode });
}

export async function deleteWorkspace(id: string) {
  return writePost('/api/workspace-delete', { id });
}

export async function fetchProjects(workspace: string) {
  return readGet(`/api/projects?workspace=${encodeURIComponent(workspace)}&_=${Date.now()}`);
}

export async function createProject(workspace: string, title: string, description = '') {
  return writePost('/api/project-create', { workspace, title, description });
}

export async function updateProject(
  workspace: string,
  project: string,
  title: string,
  description = '',
) {
  return writePost('/api/project-update', { workspace, project, title, description });
}

export async function deleteProject(workspace: string, project: string) {
  return writePost('/api/project-delete', { workspace, project });
}
