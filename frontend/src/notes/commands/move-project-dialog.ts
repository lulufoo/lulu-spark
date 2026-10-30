import { flushSync } from 'react-dom';
import { getEntryId, state } from '../state/host.ts';
import * as api from '../../host/api.ts';
import { moveProjectOpenStore } from '../state/dialog-open.ts';
import {
  emptyMoveProjectView,
  moveProjectViewStore,
  patchMoveProject,
  type MoveProjectItem,
  type MoveWorkspaceItem,
} from '../state/move-project.ts';
import type { NoteEntry } from '../state/types.ts';

export { moveProjectOpenStore };
export { moveProjectViewStore } from '../state/move-project.ts';

type ProjectRec = { project_id?: string; project_name?: string; description?: string; is_default?: boolean };
type WorkspaceRec = { workspace_id?: string; workspace_name?: string };

let currentEntry: NoteEntry | null = null;

function dualWriteMoveOpen(open: boolean) {
  const dialog = document.getElementById('move-project-dialog');
  if (!dialog) return;
  if (open) dialog.classList.add('open');
  else dialog.classList.remove('open');
}

function mapWorkspaces(rows: WorkspaceRec[] | undefined): MoveWorkspaceItem[] {
  return (rows || [])
    .map((row) => {
      const workspace_id = typeof row.workspace_id === 'string' ? row.workspace_id.trim() : '';
      if (!workspace_id) return null;
      const name =
        typeof row.workspace_name === 'string' && row.workspace_name.trim()
          ? row.workspace_name.trim()
          : workspace_id;
      return { workspace_id, workspace_name: name };
    })
    .filter((item): item is MoveWorkspaceItem => Boolean(item));
}

function mapProjects(rows: ProjectRec[] | undefined, hideProjectId: string): MoveProjectItem[] {
  return (rows || [])
    .map((row) => {
      const id = typeof row.project_id === 'string' ? row.project_id.trim() : '';
      if (!id || id === hideProjectId) return null;
      return {
        project_id: id,
        project_name:
          typeof row.project_name === 'string' && row.project_name.trim() ? row.project_name.trim() : id,
        description: typeof row.description === 'string' ? row.description : '',
        is_default: row.is_default === true,
      };
    })
    .filter((item): item is MoveProjectItem => Boolean(item))
    .sort((a, b) => a.project_name.localeCompare(b.project_name, 'en', { sensitivity: 'base' }));
}

async function loadProjects(workspaceId: string) {
  const { currentProject, sourceWorkspaceId } = moveProjectViewStore.getSnapshot();
  const hide = workspaceId === sourceWorkspaceId ? currentProject : '';
  patchMoveProject({
    result: 'Loading project list…',
    resultKind: 'loading',
    items: [],
  });
  try {
    const data = (await api.fetchProjects(workspaceId)) as { projects?: ProjectRec[] };
    const items = mapProjects(data.projects, hide);
    patchMoveProject({
      items,
      result: items.length
        ? `Current project: ${currentProject} — choose a project`
        : 'No other project in this workspace',
      resultKind: 'hint',
    });
  } catch (e) {
    console.error('[move-project] render error', e);
    patchMoveProject({
      result: 'Render failed: ' + (e as Error).message,
      resultKind: 'err',
    });
  }
}

export async function selectMoveWorkspace(workspaceId: string) {
  if (!workspaceId || moveProjectViewStore.getSnapshot().busy) return;
  patchMoveProject({ selectedWorkspaceId: workspaceId });
  await loadProjects(workspaceId);
}

export async function openMoveProjectDialog(entry: NoteEntry | null | undefined) {
  if (!entry) return;

  const currentProject = entry.project_id || 'unknown';
  const sourceWorkspaceId = entry.workspace_id || state.workspaces.current;
  currentEntry = entry;
  moveProjectViewStore.set({
    ...emptyMoveProjectView(),
    currentProject,
    sourceWorkspaceId,
    selectedWorkspaceId: sourceWorkspaceId,
    result: 'Loading project list…',
    resultKind: 'loading',
  });
  flushSync(() => {
    moveProjectOpenStore.set(true);
  });
  dualWriteMoveOpen(true);

  try {
    const data = (await api.fetchWorkspaces()) as { workspaces?: WorkspaceRec[] };
    const workspaces = mapWorkspaces(data.workspaces);
    const selected =
      workspaces.some((w) => w.workspace_id === sourceWorkspaceId)
        ? sourceWorkspaceId
        : workspaces[0]?.workspace_id || sourceWorkspaceId;
    patchMoveProject({ workspaces, selectedWorkspaceId: selected });
    await loadProjects(selected);
  } catch (e) {
    console.error('[move-project] workspaces error', e);
    patchMoveProject({
      result: 'Render failed: ' + (e as Error).message,
      resultKind: 'err',
    });
  }
}

export function closeMoveProjectDialog() {
  moveProjectOpenStore.set(false);
  moveProjectViewStore.set(emptyMoveProjectView());
  dualWriteMoveOpen(false);
  currentEntry = null;
}

export async function doMoveProject(newProject: string) {
  const entry = currentEntry;
  if (!entry) return;
  const targetWorkspaceId = moveProjectViewStore.getSnapshot().selectedWorkspaceId;

  patchMoveProject({
    result: 'Moving…',
    resultKind: 'hint',
    busy: true,
  });

  try {
    const id = getEntryId(entry);
    if (!id) throw new Error('Entry id not found');
    const data = (await api.moveToProject(id, newProject, targetWorkspaceId || undefined)) as {
      ok?: boolean;
      error?: string;
    };
    if (!data.ok) throw new Error(data.error || 'failed');

    patchMoveProject({
      result: `✓ Moved to ${newProject}`,
      resultKind: 'ok',
    });

    setTimeout(() => {
      closeMoveProjectDialog();
      document.dispatchEvent(new CustomEvent('cta:reload'));
    }, 800);
  } catch (e) {
    patchMoveProject({
      result: `✗ ${(e as Error).message}`,
      resultKind: 'err',
      busy: false,
    });
  }
}
