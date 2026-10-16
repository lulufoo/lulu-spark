import { invoke } from './transport.ts';

/** Knowledge document identity plus tree location. */
export type ResolvedKnowledgeDoc = { id: string; ok: true; topic: string; path: string };

/** Rust validates a `knowledge:<id>` target. Rejects when the id is unknown or not in the library. */
export async function resolveKnowledgeForOpen(id: string): Promise<ResolvedKnowledgeDoc> {
  return invoke('resolve_knowledge_for_open', { id }) as Promise<ResolvedKnowledgeDoc>;
}
