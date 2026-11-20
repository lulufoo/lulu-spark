import * as api from '../../host/api.ts';
import { state } from '../state/host.ts';
import { emptyFolderMove, folderMoveStore } from '../state/folder-move.ts';
import { refreshNotesIndex } from './reload-index.ts';

/** Same slice of a drag event the tree handlers need; keeps React out of the command module. */
type DragEventLike = {
  dataTransfer: { effectAllowed?: string; dropEffect?: string; setData(type: string, value: string): void } | null;
  preventDefault(): void;
  stopPropagation(): void;
};

/** The folder path of a tree node key, `<project id>/<sub>/<sub>` → `<sub>/<sub>`. */
function folderPathOf(nodeKey: string): string {
  return nodeKey.split('/').slice(1).join('/');
}

/** A folder drops on another folder of its project, never on itself or into its own subtree. */
export function folderAcceptsFolderDrop(path: string, nodeKey: string): boolean {
  if (!path || !nodeKey) return false;
  const nodeFolder = folderPathOf(nodeKey);
  if (!nodeFolder) return true;
  return nodeFolder !== path && !nodeFolder.startsWith(`${path}/`);
}

/** Start dragging the folder at `nodeKey`. */
export function startFolderMove(nodeKey: string, event: DragEventLike) {
  const path = folderPathOf(nodeKey);
  if (!path) return;
  folderMoveStore.set({ path, fromKey: nodeKey, over: '' });
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move';
    event.dataTransfer.setData('text/plain', path);
  }
}

export function endFolderMove() {
  folderMoveStore.set(emptyFolderMove());
}

/** Highlight a folder row that can take the dragged folder. */
export function overFolderTarget(nodeKey: string, event: DragEventLike) {
  const { path, fromKey, over } = folderMoveStore.getSnapshot();
  if (!path || !folderAcceptsFolderDrop(path, nodeKey)) return;
  event.preventDefault();
  event.stopPropagation();
  if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
  if (over !== nodeKey) folderMoveStore.set({ path, fromKey, over: nodeKey });
}

export function leaveFolderTarget(nodeKey: string) {
  const snap = folderMoveStore.getSnapshot();
  if (snap.over === nodeKey) folderMoveStore.set({ ...snap, over: '' });
}

/** The project sidebar background is the drop target for the project root. */
export function overRootDrop(event: DragEventLike) {
  const { path } = folderMoveStore.getSnapshot();
  if (!path) return;
  event.preventDefault();
  if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
}

export async function dropFolderOnRoot(event: DragEventLike) {
  event.preventDefault();
  const snap = folderMoveStore.getSnapshot();
  endFolderMove();
  await moveFolderTo(snap.path, snap.fromKey, '');
}

export async function dropFolderOn(nodeKey: string, event: DragEventLike) {
  event.preventDefault();
  event.stopPropagation();
  const snap = folderMoveStore.getSnapshot();
  endFolderMove();
  await moveFolderTo(snap.path, snap.fromKey, folderPathOf(nodeKey));
}

/** Move through the backend, then read the index again so tree, database, and files agree. A failure leaves everything in place and says so. */
async function moveFolderTo(path: string, fromKey: string, target: string) {
  if (!path) return;
  const project = fromKey.split('/')[0];
  try {
    await api.moveFolder({ workspace_id: state.workspaces.current, project_id: project, path, target });
    await refreshNotesIndex();
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    alert(`Could not move the folder: ${message}`);
  }
}
