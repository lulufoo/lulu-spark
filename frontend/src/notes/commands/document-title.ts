import * as api from '../../host/api.ts';
import { getEntryId, notifyState, state } from '../state/host.ts';
import type { HostNoteEntry } from '../../host/snapshot-types.ts';

// Keep writes for a document in order even when its editor is unmounted and reopened.
const pendingTitles = new Map<string, Promise<string>>();

export function hasPendingDocumentTitleSave(id: string): boolean {
  return pendingTitles.has(id);
}

/** Save metadata separately from the Markdown body and filename. */
export async function saveDocumentTitle(id: string, value: string): Promise<string> {
  const title = value.trim();
  if (!title) throw new Error('Title cannot be empty');
  const save = async () => {
    await api.setDocTitle(id, title);

    // Keep references stable: replacing the reader entry repaints an embedded Sketch.
    const update = (entry: HostNoteEntry | null | undefined) => {
      if (getEntryId(entry) === id && entry) entry.title = title;
    };
    update(state.viewer.entry);
    for (const entry of Object.values(state.index.data || {})) update(entry);
    for (const group of [...state.index.groupedByDate, ...state.index.filteredGroups]) {
      for (const item of group.entries) if (item.id === id) item.entry.title = title;
    }
    for (const titles of state.index.titleCache.values()) titles.delete(id);
    notifyState();
    return title;
  };
  const previous = pendingTitles.get(id);
  const pending = previous ? previous.catch(() => undefined).then(save) : save();
  pendingTitles.set(id, pending);
  try {
    return await pending;
  } finally {
    if (pendingTitles.get(id) === pending) pendingTitles.delete(id);
  }
}
