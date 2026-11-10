import * as api from '../../host/api.ts';
import { getEntryId } from '../../host/state.ts';
import { parseHash } from '../../router/index.ts';
import { notifyState, state } from '../state/host.ts';
import { dirCreateStore } from '../state/dir-create.ts';
import { folderCreateStore } from '../state/folder-create.ts';
import { folderRenameStore } from '../state/folder-rename.ts';
import { fileRenameStore } from '../state/file-rename.ts';
import { refreshNotesIndex } from './reload-index.ts';
import { isDirectoryView } from './workspace.ts';

function startFileRename(id: string, project: string, path: string) {
  const name = path.split('/').pop() ?? '';
  if (!id || !name) return;
  fileRenameStore.set((current) => ({
    id,
    project,
    path,
    name,
    saving: false,
    tick: current.tick + 1,
  }));
}

function enterCanStartRename(target: EventTarget | null): boolean {
  const el = target instanceof Element ? target : target instanceof Node ? target.parentElement : null;
  if (!el) return true;
  if (el.closest('input, textarea, select, [contenteditable="true"]')) return false;
  if (el.closest('[data-role="folder-menu"], [role="dialog"]')) return false;
  return true;
}

/**
 * Enter with an open document edits its file name.
 * A text field or an open dialog keeps its own Enter.
 */
export function renameSelectedFile(target: EventTarget | null): boolean {
  const renaming = fileRenameStore.getSnapshot();
  const folder = folderRenameStore.getSnapshot();
  const creating = folderCreateStore.getSnapshot();
  if (renaming.id || renaming.saving || folder.project || folder.saving) return false;
  if (creating.project || creating.menu || creating.saving) return false;
  if (dirCreateStore.getSnapshot().project) return false;
  if (!isDirectoryView() || parseHash().name !== 'spark') return false;
  if (state.viewer?.editing || state.viewer?.createSession) return false;
  if (state.viewer?.outletMode !== 'open') return false;
  const entry = state.viewer?.entry;
  const id = getEntryId(entry);
  const project = state.ui.activeTopic;
  const path = entry?.raw_path;
  if (!id || !project || entry?.project_id !== project || !path) return false;
  if (!enterCanStartRename(target)) return false;
  startFileRename(id, project, path);
  return fileRenameStore.getSnapshot().id === id;
}

export function setFileRenameName(name: string) {
  fileRenameStore.set((prev) => ({ ...prev, name }));
}

/** Drop the editor. Nothing is written. */
export function cancelFileRename() {
  const prev = fileRenameStore.getSnapshot();
  if (prev.saving) return;
  fileRenameStore.set({ id: '', project: '', path: '', name: '', saving: false, tick: prev.tick });
}

/**
 * Enter or blur. A new name moves this document's files, then the tree reads the index again.
 * An empty name, or the current name, leaves the file as it is.
 */
export async function commitFileRename() {
  const snap = fileRenameStore.getSnapshot();
  if (!snap.id || snap.saving) return;
  const name = snap.name.trim();
  const current = snap.path.split('/').pop() ?? '';
  if (!name || name === current) {
    cancelFileRename();
    return;
  }
  const { id, project } = snap;
  const workspaceId = state.workspaces.current;
  fileRenameStore.set({ ...snap, name, saving: true });
  try {
    await api.renameDoc({ workspace_id: workspaceId, project_id: project, id, name });
    fileRenameStore.set({ id: '', project: '', path: '', name: '', saving: false, tick: snap.tick });
    if (workspaceId === state.workspaces.current) await refreshNotesIndex();
    const entry = state.index.data?.[id];
    if (entry && getEntryId(state.viewer.entry) === id) {
      state.viewer.entry = { ...state.viewer.entry, ...entry };
      notifyState();
    }
  } catch (err) {
    fileRenameStore.set((prev) => ({ ...prev, saving: false }));
    const message = err instanceof Error ? err.message : String(err);
    alert(`Could not rename the file: ${message}`);
  }
}
