import { resolveNoteForOpen } from '../../host/api.ts';
import { state } from '../../host/state.ts';
import { refreshNotesIndex } from '../../notes/commands/reload-index.ts';
import { showActionError } from './hub.ts';

/** Chat links that open a note: `[title](note:<32-hex id>)`. Written by the model. */
const NOTE_HREF = /^note:([0-9a-f]{32})$/;

export function noteIdFromHref(href: string | null | undefined): string | null {
  const match = NOTE_HREF.exec((href ?? '').trim());
  return match ? match[1] : null;
}

function hasEntry(id: string): boolean {
  const data = state.index.data || {};
  return Boolean(data[id] || Object.values(data).some((entry) => entry._id === id || entry.id === id));
}

function reportOpenFailure(reason: unknown) {
  const detail = reason instanceof Error ? reason.message : String(reason);
  showActionError({ message: `Cannot open note: ${detail}` });
}

/** Rust validates the id; the Notes list must hold the entry before the shared open event. */
export async function openNoteById(id: string): Promise<void> {
  try {
    const note = await resolveNoteForOpen(id);
    if (!hasEntry(id)) await refreshNotesIndex();
    if (!hasEntry(id)) throw new Error('not in the notes list yet');
    document.dispatchEvent(
      new CustomEvent('cta:open-entry', { detail: { id: note.id || id } }),
    );
  } catch (err) {
    reportOpenFailure(err);
  }
}

/** Delegated click for rendered chat markdown. Only `note:` links are intercepted. */
export function onNoteLinkClick(event: {
  target: EventTarget | null;
  preventDefault(): void;
}): void {
  const target = event.target;
  if (!(target instanceof Element)) return;
  const anchor = target.closest('a[href^="note:"]');
  if (!anchor) return;
  event.preventDefault();
  const id = noteIdFromHref(anchor.getAttribute('href'));
  if (!id) {
    reportOpenFailure('invalid note link');
    return;
  }
  void openNoteById(id);
}
