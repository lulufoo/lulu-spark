import { getActivePath } from '../../knowledge/state/path.ts';
import { notesFileRelPath } from '../../host/constants.ts';
import { openPathInChat } from '../../home/commands/open-in-chat.ts';

export async function openNoteInChat(
  entry: { common_path?: string; translations?: { zh?: string } } | null,
  lang: string | null,
  layer: string,
  workbenchRoot: string,
  button?: HTMLButtonElement | null,
) {
  if (!entry) return;
  const activePath = getActivePath(entry, lang || '', layer) || '';
  const relPath = notesFileRelPath(layer, activePath);
  const fullPath = workbenchRoot ? `${workbenchRoot}/${relPath}` : relPath;
  if (button) button.disabled = true;
  try {
    await openPathInChat(fullPath);
  } finally {
    if (button) button.disabled = false;
  }
}
