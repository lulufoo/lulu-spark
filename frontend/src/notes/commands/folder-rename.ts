import * as api from '../../host/api.ts';
import { notifyState, state } from '../state/host.ts';
import { dirKeyOf, dirTreeStore, isDirKey, nodeKeyOf, pathKeys } from '../state/dir-tree.ts';
import { folderCreateStore } from '../state/folder-create.ts';
import { folderRenameStore } from '../state/folder-rename.ts';
import { refreshNotesIndex } from './reload-index.ts';
import { applyListFilters } from './sidebar.ts';
import { rememberNode } from './viewer-state.ts';

/** The menu's Rename: edit that folder's own name in the tree. */
export function beginFolderRename() {
  const prev = folderCreateStore.getSnapshot();
  const project = state.ui.activeTopic;
  const menu = prev.menu;
  if (!project || !menu?.canRename || prev.saving || folderRenameStore.getSnapshot().saving) return;
  const path = menu.parent;
  const name = path.split('/').pop() ?? '';
  folderCreateStore.set({ ...prev, project: '', parent: '/', name: '', menu: null });
  folderRenameStore.set((current) => ({
    project,
    path,
    name,
    saving: false,
    tick: current.tick + 1,
  }));
}

export function setFolderRenameName(name: string) {
  folderRenameStore.set((prev) => ({ ...prev, name }));
}

/** Drop the editor. Nothing is written. */
export function cancelFolderRename() {
  const prev = folderRenameStore.getSnapshot();
  if (prev.saving) return;
  folderRenameStore.set({ project: '', path: '', name: '', saving: false, tick: prev.tick });
}

/**
 * Enter or blur. A new name moves each document in the folder, then the tree reads the index again.
 * An empty name, or the current name, leaves the folder as it is.
 */
export async function commitFolderRename() {
  const snap = folderRenameStore.getSnapshot();
  if (!snap.project || snap.saving) return;
  const name = snap.name.trim();
  const current = snap.path.split('/').pop() ?? '';
  if (!name || name === current) {
    cancelFolderRename();
    return;
  }
  const { project, path } = snap;
  const workspaceId = state.workspaces.current;
  folderRenameStore.set({ ...snap, name, saving: true });
  try {
    await api.renameFolder({ workspace_id: workspaceId, project_id: project, path, name });
    folderRenameStore.set({ project: '', path: '', name: '', saving: false, tick: snap.tick });
    if (workspaceId === state.workspaces.current) await refreshNotesIndex();
    const parent = path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : '';
    retargetFolder(project, path, parent ? `${parent}/${name}` : name);
  } catch (err) {
    folderRenameStore.set((prev) => ({ ...prev, saving: false }));
    const message = err instanceof Error ? err.message : String(err);
    alert(`Could not rename the folder: ${message}`);
  }
}

/** Keep the open list and the open branches on the folder after its path changes. */
function retargetFolder(project: string, from: string, to: string) {
  const fromNode = `${project}/${from}`;
  const toNode = `${project}/${to}`;
  const mapKey = (key: string) =>
    key === fromNode || key.startsWith(`${fromNode}/`) ? toNode + key.slice(fromNode.length) : key;
  const next = dirTreeStore.getSnapshot().expanded.map(mapKey);
  for (const key of pathKeys(toNode)) if (!next.includes(key)) next.push(key);
  dirTreeStore.set({ expanded: next });
  if (!isDirKey(state.ui.activeDate)) return;
  const key = nodeKeyOf(state.ui.activeDate);
  if (key !== fromNode && !key.startsWith(`${fromNode}/`)) return;
  const node = mapKey(key);
  state.ui.activeDate = dirKeyOf(node);
  applyListFilters();
  notifyState();
  rememberNode(project, node);
}
