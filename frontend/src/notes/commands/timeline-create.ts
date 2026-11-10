import * as api from '../../host/api.ts';
import { state } from '../state/host.ts';
import { timelineCreateStore } from '../state/timeline-create.ts';
import { landOnNewDocument } from './land-on-document.ts';
import { refreshNotesIndex } from './reload-index.ts';

/** Show the title card for `project`. A card already open for that project keeps its title. */
export function beginTimelineCreate(project: string) {
  const prev = timelineCreateStore.getSnapshot();
  if (prev.saving || !project) return;
  timelineCreateStore.set({
    project,
    title: prev.project === project ? prev.title : '',
    saving: false,
    tick: prev.tick + 1,
  });
}

export function setTimelineCreateTitle(title: string) {
  timelineCreateStore.set((prev) => ({ ...prev, title }));
}

/** Drop the card. Nothing is written. */
export function cancelTimelineCreate() {
  const prev = timelineCreateStore.getSnapshot();
  if (prev.saving) return;
  timelineCreateStore.set({ project: '', title: '', saving: false, tick: prev.tick });
}

/**
 * Enter or blur: a title creates an empty document in the card's project.
 * A blank title removes the card.
 */
export async function commitTimelineCreate() {
  const snap = timelineCreateStore.getSnapshot();
  if (!snap.project || snap.saving) return;
  const title = snap.title.trim();
  if (!title) {
    cancelTimelineCreate();
    return;
  }
  const project = snap.project;
  const workspaceId = state.workspaces.current;
  timelineCreateStore.set({ ...snap, title, saving: true });
  try {
    const created = (await api.createNote({
      title,
      source_type: 'jot',
      workspace_id: workspaceId,
      project_id: project,
      content: '',
    })) as { id?: string };
    timelineCreateStore.set({ project: '', title: '', saving: false, tick: snap.tick });
    const id = created?.id;
    if (id && workspaceId === state.workspaces.current) {
      await refreshNotesIndex();
      landOnNewDocument(id, project);
    }
  } catch (err) {
    timelineCreateStore.set((prev) => ({ ...prev, saving: false }));
    const message = err instanceof Error ? err.message : String(err);
    alert(`Could not create the document: ${message}`);
  }
}
