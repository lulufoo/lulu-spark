import type { AssistantNote } from './types.ts';

export type { AssistantNote };

export function isJotSourceType(sourceType?: string) {
  return sourceType === 'jot' || sourceType === 'note';
}

export function selectTopNotesByCreatedAt(entries: AssistantNote[]) {
  return [...entries]
    .filter((entry) => isJotSourceType(entry?.source_type))
    .sort((a, b) => String(b.created_at ?? '').localeCompare(String(a.created_at ?? '')))
    .slice(0, 3);
}
