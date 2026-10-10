import { openDraftInChat } from '../../home/commands/open-in-chat.ts';
import { slugToTitle, filenameFromPath } from '../../shared/utils.ts';
import { getEntryId } from '../state/host.ts';
import type { HostNoteEntry } from '../../host/state.ts';
import { serializeReference } from '../../home/references/index.ts';

function noteTitle(entry: HostNoteEntry) {
  if (entry.title) return String(entry.title);
  if (entry.common_path) return slugToTitle(filenameFromPath(entry.common_path));
  return 'Untitled note';
}

/** `[title](note:<id>) ` — shown as a chip in the composer; the space keeps typing after it. */
export function noteReferenceDraft(entry: HostNoteEntry, id: string) {
  const title = noteTitle(entry).replace(/\s+/g, ' ').trim();
  return `${serializeReference({ kind: 'note', id, title })} `;
}

export async function openNoteInChat(
  entry: HostNoteEntry | null,
  button?: HTMLButtonElement | null,
) {
  if (!entry) return;
  const sourceId = getEntryId(entry);
  if (!sourceId) throw new Error('Missing source');
  if (button) button.disabled = true;
  try {
    await openDraftInChat(noteReferenceDraft(entry, sourceId));
  } finally {
    if (button) button.disabled = false;
  }
}
