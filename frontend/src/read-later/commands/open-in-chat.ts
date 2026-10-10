import { openDraftInChat } from '../../home/commands/open-in-chat.ts';
import { serializeReference } from '../../home/references/index.ts';
import type { ReadLaterEntry } from '../state/types.ts';

/** `[title](read-later:<id>) ` — shown as a chip in the composer; the space keeps typing after it. */
export function readLaterReferenceDraft(entry: ReadLaterEntry) {
  const id = String(entry.id || '').trim();
  if (!id) throw new Error('Missing source');
  const title = String(entry.title || entry.url || 'Untitled').replace(/\s+/g, ' ').trim();
  return `${serializeReference({ kind: 'read-later', id, title })} `;
}

export async function openReadLaterInChat(
  entry: ReadLaterEntry,
  button?: HTMLButtonElement | null,
) {
  const draft = readLaterReferenceDraft(entry);
  if (button) button.disabled = true;
  try {
    await openDraftInChat(draft);
  } finally {
    if (button) button.disabled = false;
  }
}
