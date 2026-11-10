import * as api from '../../host/api.ts';
import { notifyState, state } from '../state/host.ts';
import { expandDirNodes, pathKeys } from '../state/dir-tree.ts';
import { folderCreateStore } from '../state/folder-create.ts';
import { refreshNotesIndex } from './reload-index.ts';
import { applyListFilters } from './sidebar.ts';
import { rememberNode } from './viewer-state.ts';

/** A folder path of three segments is the deepest one that can still be stored. */
export function folderCanHaveChild(parent: string): boolean {
  if (!parent || parent === '/') return true;
  return parent.split('/').filter(Boolean).length < 3;
}

/** `ideas/plans/q1` → `plans/q1`. A project key is the project root. */
export function folderPathOf(nodeKey: string): string {
  const slash = nodeKey.indexOf('/');
  return slash === -1 ? '/' : nodeKey.slice(slash + 1);
}

export function openFolderMenu(parent: string, x: number, y: number) {
  const project = state.ui.activeTopic;
  if (!project || !folderCanHaveChild(parent)) return;
  folderCreateStore.set((prev) => ({ ...prev, menu: { parent, x, y } }));
}

export function closeFolderMenu() {
  folderCreateStore.set((prev) => (prev.menu ? { ...prev, menu: null } : prev));
}

/** Clicking empty tree space leaves the project selected and no folder listed. */
export function deselectFolder() {
  const project = state.ui.activeTopic;
  if (!project || !state.ui.activeDate) return;
  state.ui.activeDate = null;
  applyListFilters();
  notifyState();
  rememberNode(project, '');
}

/** The menu's New folder: an empty name row under that folder. */
export function beginFolderCreate() {
  const prev = folderCreateStore.getSnapshot();
  const project = state.ui.activeTopic;
  if (!project || !prev.menu || prev.saving) return;
  const parent = prev.menu.parent;
  const node = parent === '/' ? project : `${project}/${parent}`;
  expandDirNodes(pathKeys(node));
  folderCreateStore.set({
    project,
    parent,
    name: '',
    saving: false,
    tick: prev.tick + 1,
    menu: null,
  });
}

export function setFolderCreateName(name: string) {
  folderCreateStore.set((prev) => ({ ...prev, name }));
}

/** Drop the row. Nothing is written. */
export function cancelFolderCreate() {
  const prev = folderCreateStore.getSnapshot();
  if (prev.saving) return;
  folderCreateStore.set({ project: '', parent: '/', name: '', saving: false, tick: prev.tick, menu: null });
}

/**
 * Enter or blur: a name creates the folder. A blank name removes the row.
 * The folder stays in the tree after the page reads the index again.
 */
export async function commitFolderCreate() {
  const snap = folderCreateStore.getSnapshot();
  if (!snap.project || snap.saving) return;
  const name = snap.name.trim();
  if (!name) {
    cancelFolderCreate();
    return;
  }
  const project = snap.project;
  const parent = snap.parent;
  const workspaceId = state.workspaces.current;
  folderCreateStore.set({ ...snap, name, saving: true });
  try {
    await api.createFolder({
      workspace_id: workspaceId,
      project_id: project,
      name,
      ...(parent !== '/' ? { parent } : {}),
    });
    folderCreateStore.set({ project: '', parent: '/', name: '', saving: false, tick: snap.tick, menu: null });
    if (workspaceId === state.workspaces.current) await refreshNotesIndex();
  } catch (err) {
    folderCreateStore.set((prev) => ({ ...prev, saving: false }));
    const message = err instanceof Error ? err.message : String(err);
    alert(`Could not create the folder: ${message}`);
  }
}
