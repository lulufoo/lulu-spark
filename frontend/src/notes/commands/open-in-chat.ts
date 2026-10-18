import { openDraftInChat } from '../../home/commands/open-in-chat.ts';
import { getEntryId, state } from '../state/host.ts';
import type { HostNoteEntry } from '../../host/state.ts';
import { serializeReference } from '../../home/references/index.ts';
import { formatReferenceText, writeClipboardText } from '../../shared/reference-text.ts';

function noteTitle(entry: HostNoteEntry, id: string) {
  if (entry.title) return String(entry.title);
  const date = String(entry.created_at || '').slice(0, 8);
  const cached = /^\d{8}$/.test(date)
    ? String(state.index.titleCache.get(date)?.get(id) || '').trim()
    : '';
  if (cached) return cached;
  const viewing = state.viewer.entry;
  const sameNote = Boolean(viewing && (getEntryId(viewing) === id || viewing === entry));
  const fromBody = sameNote
    ? (String(state.viewer.rawText || '').match(/^#\s+(.+)/m)?.[1]?.trim() || '')
    : '';
  if (fromBody) return fromBody;
  return 'Untitled note';
}

/** Current name of the workspace that holds `entry`; the workspace id when the name is unknown. */
function workspaceNameOf(entry: HostNoteEntry) {
  const id = String(entry.workspace_id || state.workspaces.current);
  return state.workspaces.list.find((w) => w.workspace_id === id)?.workspace_name || id;
}

/** `[Workspace name：Title](spark-doc:<id>) ` — shown as a chip in the composer; the space keeps typing after it. */
export function noteReferenceDraft(entry: HostNoteEntry, id: string) {
  const title = noteTitle(entry, id).replace(/\s+/g, ' ').trim();
  return `${serializeReference({ kind: 'doc', id, title: `${workspaceNameOf(entry)}：${title}` })} `;
}

/** Copy `Lulu Spark doc` / title / id for an outside tool. Resolves false when nothing was copied. */
export async function copyNoteReference(entry: HostNoteEntry | null) {
  if (!entry) return false;
  const id = getEntryId(entry);
  if (!id) return false;
  return writeClipboardText(formatReferenceText({ kind: 'doc', id, title: noteTitle(entry, id) }));
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
