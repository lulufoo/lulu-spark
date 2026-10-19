import { flushSync } from 'react-dom';
import { getEntryId, state } from '../state/host.ts';
import * as api from '../../host/api.ts';
import { moveProjectOpenStore } from '../state/dialog-open.ts';
import {
  emptyMoveProjectView,
  moveProjectViewStore,
  patchMoveProject,
  type MoveProjectItem,
} from '../state/move-project.ts';
import type { NoteEntry } from '../state/types.ts';

export { moveProjectOpenStore };
export { moveProjectViewStore } from '../state/move-project.ts';

type ProjectRec = { project_id?: string; project_name?: string; description?: string; is_default?: boolean };

let currentEntry: NoteEntry | null = null;

function dualWriteMoveOpen(open: boolean) {
  const dialog = document.getElementById('move-project-dialog');
  if (!dialog) return;
  if (open) dialog.classList.add('open');
  else dialog.classList.remove('open');
}

async function loadProjects(currentProject: string) {
  patchMoveProject({
    result: 'Loading project list…',
    resultKind: 'loading',
    items: [],
    busy: false,
  });

  try {
    const data = (await api.fetchProjects(state.workspaces.current)) as { projects?: ProjectRec[] };
    const items = (data.projects || [])
      .map((c) => {
        const id = typeof c.project_id === 'string' ? c.project_id.trim() : '';
        if (!id || id === currentProject) return null;
        return {
          project_id: id,
          project_name:
            typeof c.project_name === 'string' && c.project_name.trim() ? c.project_name.trim() : id,
          description: typeof c.description === 'string' ? c.description : '',
          is_default: c.is_default === true,
        };
      })
      .filter((item): item is MoveProjectItem => Boolean(item))
      .sort((a, b) => a.project_name.localeCompare(b.project_name, 'en', { sensitivity: 'base' }));
    patchMoveProject({
      items,
      result: `Current project: ${currentProject} — choose target`,
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

export async function openMoveProjectDialog(entry: NoteEntry | null | undefined) {
  if (!entry) return;

  const currentProject = entry.project_id || 'unknown';
  currentEntry = entry;
  moveProjectViewStore.set({
    ...emptyMoveProjectView(),
    currentProject,
    result: 'Loading project list…',
    resultKind: 'loading',
  });
  flushSync(() => {
    moveProjectOpenStore.set(true);
  });
  dualWriteMoveOpen(true);

  await loadProjects(currentProject);
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

  patchMoveProject({
    result: 'Moving…',
    resultKind: 'hint',
    busy: true,
  });

  try {
    const id = getEntryId(entry);
    if (!id) throw new Error('Entry id not found');
    const data = (await api.moveToProject(id, newProject)) as { ok?: boolean; error?: string };
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
