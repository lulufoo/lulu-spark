import { fetchIndex } from '../../host/api.ts';
import { normalizeKnowledgeIndex } from '../../knowledge/state/index.ts';
import { selectTopNotesByCreatedAt, type AssistantNote } from '../state/selectors.ts';

export type { AssistantNote };
export { selectTopNotesByCreatedAt };

export async function loadAssistantNotes() {
  const data = await fetchIndex();
  const map = normalizeKnowledgeIndex(data);
  return Object.values(map) as AssistantNote[];
}
