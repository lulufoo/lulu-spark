import { fetchIndex } from '../../host/api.ts';
import { normalizeCorpusIndex } from '../../corpus/state/index.ts';
import { selectTopNotesByCreatedAt, type AssistantNote } from '../state/selectors.ts';

export type { AssistantNote };
export { selectTopNotesByCreatedAt };

export async function loadAssistantNotes() {
  const data = await fetchIndex();
  const map = normalizeCorpusIndex(data);
  return Object.values(map) as AssistantNote[];
}
