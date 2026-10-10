import type { HostNoteEntry } from '../snapshot-types.ts';
import { invoke } from './transport.ts';

export type ResolvedNote = HostNoteEntry & { id: string; ok: true };

/** Rust validates a `note:<id>` target. Rejects when the id is malformed or unknown. */
export async function resolveNoteForOpen(id: string): Promise<ResolvedNote> {
  return invoke('resolve_note_for_open', { id }) as Promise<ResolvedNote>;
}
