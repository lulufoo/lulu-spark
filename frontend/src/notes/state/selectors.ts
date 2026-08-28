import type { AssistantNote } from './types.ts';

export type { AssistantNote };

export function selectTopNotesByCreatedAt(entries: AssistantNote[]) {
  return [...entries]
    .filter((entry) => entry?.source_type === 'note')
    .sort((a, b) => String(b.created_at ?? '').localeCompare(String(a.created_at ?? '')))
    .slice(0, 3);
}
