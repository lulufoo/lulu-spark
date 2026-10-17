import { state } from '../state/host.ts';
import { dateKeyOf } from '../../shared/utils.ts';
import { dirKeyOf, nodeKeyOfEntry } from '../state/dir-tree.ts';
import { selectDate, selectTopic } from './sidebar.ts';
import { isDirectoryView } from './workspace.ts';

/**
 * After a document was created: select its Project and show it where it lives, so the person
 * lands on it and not back on All. Directory view → the folder it is in; timeline → its day.
 * Returns the group key (folder or day) to open it under, or '' when it is not in the index.
 */
export function landOnNewDocument(id: string, project: string): string {
  const entry = state.index.data?.[id];
  if (!entry) return '';
  if (isDirectoryView()) {
    const key = dirKeyOf(nodeKeyOfEntry(entry));
    selectDate(key);
    return key;
  }
  selectTopic(project || null);
  const day = dateKeyOf(entry.created_at);
  selectDate(day);
  return day;
}
