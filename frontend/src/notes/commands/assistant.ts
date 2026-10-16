import { fetchIndex } from '../../host/api.ts';
import { normalizeDocIndex } from '../state/doc-index.ts';
import { state } from '../state/host.ts';
import { selectTopNotesByCreatedAt, type AssistantNote } from '../state/selectors.ts';

export type { AssistantNote };
export { selectTopNotesByCreatedAt };

export async function loadAssistantNotes() {
  const data = await fetchIndex(state.workspaces.current);
  const map = normalizeDocIndex(data);
  return Object.values(map) as AssistantNote[];
}
