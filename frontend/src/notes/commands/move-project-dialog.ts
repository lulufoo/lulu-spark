import { flushSync } from 'react-dom';
import { getEntryId } from '../state/host.ts';
import * as api from '../../host/api.ts';
import { moveProjectOpenStore } from '../state/dialog-open.ts';
import { emptyMoveProjectView, moveProjectViewStore, patchMoveProject } from '../state/move-project.ts';
import type { NoteEntry } from '../state/types.ts';

export { moveProjectOpenStore };
export { moveProjectViewStore } from '../state/move-project.ts';

type TopicRec = { dir?: string; repo?: string; description?: string };

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

  let projects: string[] = [];
  const topicsMap: Record<string, string> = {};
  try {
    const data = (await api.fetchTopics()) as { topics?: TopicRec[] };
    for (const t of data.topics || []) {
      const key = t.dir || (t.repo ? t.repo.split('/').pop() : null);
      if (key) topicsMap[key] = t.description || '';
    }
    projects = (data.topics || [])
      .map((t) => t.dir || (t.repo ? t.repo.split('/').pop() : null))
      .filter((proj): proj is string => Boolean(proj));
    projects.sort((a, b) => a.localeCompare(b, 'en', { sensitivity: 'base' }));
  } catch (e) {
    patchMoveProject({
      result: 'Failed to load: ' + (e as Error).message,
      resultKind: 'err',
    });
    return;
  }

  try {
    const items = projects
      .filter((proj) => proj !== currentProject)
      .map((proj) => ({ proj, desc: topicsMap[proj] || '' }));
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

  const currentProject = entry.common_path.split('/')[0];
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
