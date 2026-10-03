import * as api from '../../host/api.ts';
import { notifyState, state } from '../../host/state.ts';
import type { HostNoteEntry } from '../../host/snapshot-types.ts';
import { normalizeKnowledgeIndex } from '../../knowledge/state/index.ts';
import { applyListFilters, buildGroups } from './sidebar.ts';

/** Replace the notes snapshot from Host `index.json`. Keeps topic/tag/date. */
export async function refreshNotesIndex(): Promise<boolean> {
  try {
    const data = await api.fetchIndex();
    state.index.data = normalizeKnowledgeIndex(data) as Record<string, HostNoteEntry>;
    state.index.groupedByDate = buildGroups(state.index.data);
    applyListFilters();
    notifyState();
    return true;
  } catch {
    return false;
  }
}
