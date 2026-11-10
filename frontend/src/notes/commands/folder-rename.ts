import * as api from '../../host/api.ts';
import { parseHash } from '../../router/index.ts';
import { notifyState, state } from '../state/host.ts';
import { dirKeyOf, dirTreeStore, isDirKey, nodeKeyOf, pathKeys } from '../state/dir-tree.ts';
import { dirCreateStore } from '../state/dir-create.ts';
import { fileRenameStore } from '../state/file-rename.ts';
import { folderCreateStore } from '../state/folder-create.ts';
import { folderRenameStore } from '../state/folder-rename.ts';
import { refreshNotesIndex } from './reload-index.ts';
import { applyListFilters } from './sidebar.ts';
import { rememberNode } from './viewer-state.ts';
import { isDirectoryView } from './workspace.ts';

function startFolderRename(project: string, path: string) {
  const name = path.split('/').pop() ?? '';
  if (!name) return;
  folderRenameStore.set((current) => ({
    project,
    path,
    name,
    saving: false,
    tick: current.tick + 1,
  }));
}

/** The menu's Rename: edit that folder's own name in the tree. */
export function beginFolderRename() {
  const prev = folderCreateStore.getSnapshot();
  const project = state.ui.activeTopic;
  const menu = prev.menu;
  if (!project || !menu?.canRename || prev.saving || folderRenameStore.getSnapshot().saving) return;
  folderCreateStore.set({ ...prev, project: '', parent: '/', name: '', menu: null });
  startFolderRename(project, menu.parent);
}

/**
 * Enter with a listed folder opens the same name editor as Rename.
 * A text field or an open dialog keeps its own Enter.
 */
export function renameSelectedFolder(target: EventTarget | null): boolean {
  const renaming = folderRenameStore.getSnapshot();
  const creating = folderCreateStore.getSnapshot();
  const file = fileRenameStore.getSnapshot();
  if (renaming.project || renaming.saving || creating.project || creating.menu || creating.saving) return false;
  if (file.id || file.saving || dirCreateStore.getSnapshot().project) return false;
  if (!isDirectoryView() || parseHash().name !== 'spark') return false;
  const outlet = state.viewer?.outletMode;
  if (outlet === 'open' || outlet === 'create' || outlet === 'safe-empty' || state.viewer?.createSession) return false;
  const project = state.ui.activeTopic;
  const active = state.ui.activeDate;
  if (!project || !isDirKey(active)) return false;
  const node = nodeKeyOf(active);
  if (!node.startsWith(`${project}/`)) return false;
  if (!enterCanStartRename(target)) return false;
  startFolderRename(project, node.slice(project.length + 1));
  return folderRenameStore.getSnapshot().project === project;
}

function enterCanStartRename(target: EventTarget | null): boolean {
  const el = target instanceof Element ? target : target instanceof Node ? target.parentElement : null;
  if (!el) return true;
  if (el.closest('input, textarea, select, [contenteditable="true"]')) return false;
  if (el.closest('[data-role="folder-menu"], [role="dialog"]')) return false;
  return true;
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
