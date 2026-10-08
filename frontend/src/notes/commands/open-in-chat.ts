import { getActivePath } from '../../knowledge/state/path.ts';
import { notesFileRelPath } from '../../host/constants.ts';
import { openPathInChat } from '../../home/commands/open-in-chat.ts';
import { getEntryId } from '../state/host.ts';
import type { HostNoteEntry } from '../../host/state.ts';

export async function openNoteInChat(
  entry: HostNoteEntry | null,
  lang: string | null,
  layer: string,
  sparkRoot: string,
  button?: HTMLButtonElement | null,
) {
  if (!entry) return;
  const sourceId = getEntryId(entry);
  if (!sourceId) throw new Error('Missing source');
  const activePath = getActivePath(entry, lang || '', layer) || '';
  const relPath = notesFileRelPath(layer, activePath);
  const fullPath = sparkRoot ? `${sparkRoot}/${relPath}` : relPath;
  if (button) button.disabled = true;
  try {
    await openPathInChat(fullPath, { kind: 'notes', id: sourceId });
  } finally {
    if (button) button.disabled = false;
  }
}
