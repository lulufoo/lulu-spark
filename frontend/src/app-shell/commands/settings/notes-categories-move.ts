import * as api from '../../../host/api.ts';
import { announceRegistryChange } from '../../../shared/registry-events.ts';
import { enterProject } from '../../../notes/commands/restore.ts';
import { state } from '../../../host/state.ts';
import {
  notesCategoriesStore,
  patchNotesCategories,
  type NotesCategoryRow,
} from '../../state/settings/notes-categories.ts';
import { loadNotesCategories } from './notes-categories.ts';

type WorkspaceRec = { workspace_id?: unknown; workspace_name?: unknown };
type ProjectRec = {
  project_id?: unknown;
  project_name?: unknown;
  description?: unknown;
  icon?: unknown;
  is_default?: unknown;
  doc_count?: unknown;
};

function text(value: unknown, fallback = '') {
  return typeof value === 'string' && value.trim() ? value.trim() : fallback;
}

function asWorkspaces(data: unknown) {
  const rows = (data as { workspaces?: WorkspaceRec[] } | null)?.workspaces || [];
  return rows
    .map((row) => {
      const id = text(row.workspace_id);
      if (!id) return null;
      return { id, name: text(row.workspace_name, id) };
    })
    .filter((row): row is { id: string; name: string } => Boolean(row));
}

function asProjects(data: unknown, hideId: string): NotesCategoryRow[] {
  const rows = (data as { projects?: ProjectRec[] } | null)?.projects || [];
  const projects: NotesCategoryRow[] = [];
  for (const row of rows) {
    const id = text(row.project_id);
    if (!id || id === hideId) continue;
    projects.push({
      id,
      title: text(row.project_name, id),
      description: typeof row.description === 'string' ? row.description : '',
      is_default: row.is_default === true,
      icon: typeof row.icon === 'string' ? row.icon : '',
      doc_count: typeof row.doc_count === 'number' ? row.doc_count : 0,
    });
  }
  return projects;
}

function stillMoving(id: string) {
  return notesCategoriesStore.getSnapshot().moveId === id;
}

async function loadMoveProjects(workspaceId: string, sourceId: string) {
  patchNotesCategories({ moveLoading: true, moveError: '', moveProjects: [], moveTargetId: '' });
  const hide = workspaceId === notesCategoriesStore.getSnapshot().workspaceId ? sourceId : '';
  try {
    const projects = asProjects(await api.fetchProjects(workspaceId), hide);
    if (!stillMoving(sourceId)) return;
    patchNotesCategories({ moveProjects: projects, moveLoading: false });
  } catch (e) {
    if (!stillMoving(sourceId)) return;
    patchNotesCategories({
      moveLoading: false,
      moveError: (e as Error).message || String(e),
    });
  }
}

/** Open the move dialog for project `id`. The built-in project cannot be moved. */
export async function askMoveProject(id: string) {
  const snap = notesCategoriesStore.getSnapshot();
  const row = snap.rows.find((item) => item.id === id);
  if (!row || row.is_default || snap.busy || snap.moveId || snap.deleteId) return;
  patchNotesCategories({
    moveId: id,
    moveError: '',
    moveTargetId: '',
    moveWorkspaceId: snap.workspaceId,
    moveWorkspaces: [],
    moveProjects: [],
    moveLoading: true,
  });
  try {
    const workspaces = asWorkspaces(await api.fetchWorkspaces());
    if (!stillMoving(id)) return;
    const selected = workspaces.some((item) => item.id === snap.workspaceId)
      ? snap.workspaceId
      : workspaces[0]?.id || snap.workspaceId;
    patchNotesCategories({ moveWorkspaces: workspaces, moveWorkspaceId: selected });
    await loadMoveProjects(selected, id);
  } catch (e) {
    if (!stillMoving(id)) return;
    patchNotesCategories({ moveLoading: false, moveError: (e as Error).message || String(e) });
  }
}

export async function selectMoveWorkspace(workspaceId: string) {
  const snap = notesCategoriesStore.getSnapshot();
  if (!snap.moveId || snap.busy || snap.moveLoading) return;
  patchNotesCategories({ moveWorkspaceId: workspaceId });
  await loadMoveProjects(workspaceId, snap.moveId);
}

/** Remember the project the user clicked. The move waits for confirm. */
export function chooseMoveTarget(projectId: string) {
  const snap = notesCategoriesStore.getSnapshot();
  if (!snap.moveId || snap.busy || snap.moveLoading) return;
  if (!snap.moveProjects.some((row) => row.id === projectId)) return;
  patchNotesCategories({ moveTargetId: projectId, moveError: '' });
}

export function backMoveTarget() {
  const snap = notesCategoriesStore.getSnapshot();
  if (!snap.moveId || snap.busy) return;
  patchNotesCategories({ moveTargetId: '', moveError: '' });
}

export function cancelMoveProject() {
  const snap = notesCategoriesStore.getSnapshot();
  if (snap.busy || !snap.moveId) return;
  patchNotesCategories({
    moveId: '',
    moveError: '',
    moveLoading: false,
    moveWorkspaces: [],
    moveWorkspaceId: '',
    moveProjects: [],
    moveTargetId: '',
  });
}

async function refreshPage(
  settingsWorkspaceId: string,
  sourceId: string,
  targetWorkspaceId: string,
  targetProjectId: string,
  inboxId: string,
) {
  if (settingsWorkspaceId !== state.workspaces.current) return;
  const wasShown = state.ui.activeTopic === sourceId;
  const loaded = new Promise<void>((resolve) => {
    document.addEventListener('cta:index-loaded', () => resolve(), { once: true });
  });
  document.dispatchEvent(new CustomEvent('cta:reload'));
  await loaded;
  if (!wasShown) return;
  if (targetWorkspaceId === state.workspaces.current) enterProject(targetProjectId);
  else if (inboxId) enterProject(inboxId);
}

/** Move after the user confirms. A failure stays in the dialog. */
export async function confirmMoveProject(workspaceId: string): Promise<boolean> {
  const snap = notesCategoriesStore.getSnapshot();
  const sourceId = snap.moveId;
  const targetId = snap.moveTargetId;
  const targetWorkspaceId = snap.moveWorkspaceId;
  if (!sourceId || !targetId || !targetWorkspaceId || snap.busy) return false;
  const inboxId = snap.rows.find((row) => row.is_default)?.id ?? '';
  patchNotesCategories({ busy: true, moveError: '' });
  try {
    const result = await api.moveProject(workspaceId, sourceId, targetWorkspaceId, targetId);
    if (result?.error) throw new Error(result.error);
  } catch (e) {
    patchNotesCategories({ busy: false, moveError: (e as Error).message || String(e) });
    await loadNotesCategories(workspaceId);
    return false;
  }
  patchNotesCategories({
    moveId: '',
    moveError: '',
    moveLoading: false,
    moveWorkspaces: [],
    moveWorkspaceId: '',
    moveProjects: [],
    moveTargetId: '',
    busy: false,
  });
  await loadNotesCategories(workspaceId);
  announceRegistryChange();
  void refreshPage(workspaceId, sourceId, targetWorkspaceId, targetId, inboxId).catch(() => {});
  return true;
}
