import * as api from '../../host/api.ts';
import { getEntryId, notifyState, state } from '../state/host.ts';
import { emptyFileMove, fileMoveStore } from '../state/file-move.ts';
import { nodeKeyOfEntry } from '../state/dir-tree.ts';
import { refreshNotesIndex } from './reload-index.ts';
import type { NoteEntry } from '../state/types.ts';

/** The slice of a drag event the tree handlers need; keeps React out of the command module. */
type DragEventLike = {
  dataTransfer: { effectAllowed?: string; dropEffect?: string; setData(type: string, value: string): void } | null;
  preventDefault(): void;
  stopPropagation(): void;
};

/** A folder takes the drop when it belongs to the dragged document's project and is not its current folder. */
export function folderAcceptsDrop(fromKey: string, nodeKey: string): boolean {
  if (!fromKey || !nodeKey) return false;
  const project = `${fromKey.split('/')[0]}/`;
  return nodeKey.startsWith(project) && nodeKey !== fromKey;
}

/** A plain click stays a click; the webview starts the drag itself past its threshold. */
export function startFileMove(id: string, entry: NoteEntry, event: DragEventLike) {
  if (!id || !entry.raw_path) return;
  fileMoveStore.set({ ...emptyFileMove(), id, fromKey: nodeKeyOfEntry(entry) });
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move';
    event.dataTransfer.setData('text/plain', entry.raw_path);
  }
}

export function endFileMove() {
  fileMoveStore.set(emptyFileMove());
}

/** Highlight the folder row while a draggable document hovers it. */
export function overFolder(nodeKey: string, event: DragEventLike) {
  const { id, fromKey, over } = fileMoveStore.getSnapshot();
  if (!id || !folderAcceptsDrop(fromKey, nodeKey)) return;
  event.preventDefault();
  event.stopPropagation();
  if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
  if (over !== nodeKey) fileMoveStore.set({ id, fromKey, over: nodeKey });
}

export function leaveFolder(nodeKey: string) {
  const snap = fileMoveStore.getSnapshot();
  if (snap.over === nodeKey) fileMoveStore.set({ ...snap, over: '' });
}

export async function dropFileOn(nodeKey: string, event: DragEventLike) {
  event.preventDefault();
  event.stopPropagation();
  const snap = fileMoveStore.getSnapshot();
  endFileMove();
  await moveFileToFolder(snap.id, snap.fromKey, nodeKey);
}

/**
 * Move through the backend, then read the index again so the tree, the database path, and
 * the files agree. A failure leaves everything in place and says so.
 */
async function moveFileToFolder(id: string, fromKey: string, nodeKey: string) {
  if (!id || !folderAcceptsDrop(fromKey, nodeKey)) return;
  const entry = state.index.data?.[id];
  if (!entry || nodeKeyOfEntry(entry) !== fromKey) return;
  const project = fromKey.split('/')[0];
  const target = nodeKey.split('/').slice(1).join('/');
  try {
    await api.moveDocDir({ workspace_id: state.workspaces.current, project_id: project, id, target });
    await refreshNotesIndex();
    const updated = state.index.data?.[id];
    if (updated && getEntryId(state.viewer.entry) === id) {
      state.viewer.entry = { ...state.viewer.entry, ...updated };
      notifyState();
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    alert(`Could not move the file: ${message}`);
  }
}
