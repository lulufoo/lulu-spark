import * as api from '../../host/api.ts';
import { state } from '../state/host.ts';
import { expandDirNodes, pathKeys } from '../state/dir-tree.ts';
import { dirCreateStore } from '../state/dir-create.ts';
import { landOnNewDocument } from './land-on-document.ts';
import { refreshNotesIndex } from './reload-index.ts';
import { selectProject } from './sidebar.ts';

/** The title stored once, from the file name the person typed. Later edits do not copy either way. */
function titleFor(filename: string): string {
  return filename.replace(/\.md$/i, '').trim();
}

/**
 * Show a file-name row under `folder` of `project`. `/` is the project root.
 * A row already open there keeps what was typed.
 */
export function beginDirCreate(project: string, folder: string) {
  const prev = dirCreateStore.getSnapshot();
  if (prev.saving || !project) return;
  let target = folder || '/';
  if (state.ui.activeTopic !== project) {
    selectProject(project);
    target = '/';
  }
  const node = target === '/' ? project : `${project}/${target}`;
  expandDirNodes(pathKeys(node));
  const same = prev.project === project && prev.folder === target;
  dirCreateStore.set({
    project,
    folder: target,
    name: same ? prev.name : '',
    saving: false,
    tick: prev.tick + 1,
  });
}

export function setDirCreateName(name: string) {
  dirCreateStore.set((prev) => ({ ...prev, name }));
}

/** Drop the row. Nothing is written. */
export function cancelDirCreate() {
  const prev = dirCreateStore.getSnapshot();
  if (prev.saving) return;
  dirCreateStore.set({ project: '', folder: '/', name: '', saving: false, tick: prev.tick });
}

/**
 * Enter or blur: a file name creates an empty document in that folder.
 * A blank name removes the row.
 */
export async function commitDirCreate() {
  const snap = dirCreateStore.getSnapshot();
  if (!snap.project || snap.saving) return;
  const filename = snap.name.trim();
  if (!filename) {
    cancelDirCreate();
    return;
  }
  const title = titleFor(filename);
  if (!title) {
    alert('Invalid filename');
    return;
  }
  const project = snap.project;
  const folder = snap.folder;
  const workspaceId = state.workspaces.current;
  dirCreateStore.set({ ...snap, name: filename, saving: true });
  try {
    const created = (await api.createNote({
      title,
      filename,
      folder,
      source_type: 'jot',
      workspace_id: workspaceId,
      project_id: project,
      content: '',
    })) as { id?: string };
    dirCreateStore.set({ project: '', folder: '/', name: '', saving: false, tick: snap.tick });
    const id = created?.id;
    if (id && workspaceId === state.workspaces.current) {
      await refreshNotesIndex();
      landOnNewDocument(id, project);
    }
  } catch (err) {
    dirCreateStore.set((prev) => ({ ...prev, saving: false }));
    const message = err instanceof Error ? err.message : String(err);
    alert(`Could not create the document: ${message}`);
  }
}
