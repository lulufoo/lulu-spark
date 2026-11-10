import { LAYERS } from '../../host/constants.ts';
import { navigateToNote } from '../../router/index.ts';
import { state } from '../state/host.ts';
import { enterDocProject } from './restore.ts';
import { revealGroup } from './sidebar.ts';
import { entryGroupKey } from './workspace.ts';

/**
 * Open a document from the index. The sidebar lists its folder (or day) but the reader stays up.
 * `selectDate` would close the note, so a second click must not go through it.
 */
export function openIndexEntry(id: string, layer?: string): boolean {
  const data = state.index.data || {};
  const entry = data[id] ?? Object.values(data).find((item) => item._id === id || item.id === id);
  if (!entry) return false;
  enterDocProject(entry);
  const date = entryGroupKey(entry);
  revealGroup(date);
  const picked = layer || LAYERS.find((name) => entry.layers?.includes(name)) || 'raw';
  return navigateToNote({ date, note: id, layer: picked });
}
