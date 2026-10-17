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

export async function fetchDisabledWorkspaces() {
  return readGet('/api/workspaces-disabled?_=' + Date.now());
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

export async function fetchDisabledProjects(workspace: string) {
  return readGet(`/api/projects-disabled?workspace=${encodeURIComponent(workspace)}&_=${Date.now()}`);
}

export async function disableProject(workspace: string, project: string) {
  return writePost('/api/project-disable', { workspace, project });
}

export async function enableProject(workspace: string, project: string) {
  return writePost('/api/project-enable', { workspace, project });
}
