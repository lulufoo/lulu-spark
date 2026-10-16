import { fetchIndex } from '../../host/api.ts';
import { normalizeKnowledgeIndex } from '../../knowledge/state/index.ts';
import { state } from '../state/host.ts';
import { selectTopNotesByCreatedAt, type AssistantNote } from '../state/selectors.ts';

export type { AssistantNote };
export { selectTopNotesByCreatedAt };

export async function loadAssistantNotes() {
  const data = await fetchIndex(state.workspaces.current);
  const map = normalizeKnowledgeIndex(data);
  return Object.values(map) as AssistantNote[];
}
