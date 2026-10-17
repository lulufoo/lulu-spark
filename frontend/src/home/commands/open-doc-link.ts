import { resolveDocForOpen } from '../../host/api.ts';
import { state } from '../../host/state.ts';
import { refreshNotesIndex } from '../../notes/commands/reload-index.ts';
import { showWorkspace } from '../../notes/commands/workspace-switch.ts';
import { showActionError } from './hub.ts';

/** Chat links that open a document: `[Workspace name：Title](doc:<id>)`. The id is `<workspace id>-<32 hex>`
 * or a plain 32 hex from before the prefix. Only the id is read; the link text is for display. */
const DOC_HREF = /^doc:((?:[a-z0-9_]+-)?[0-9a-f]{32})$/;

export function docIdFromHref(href: string | null | undefined): string | null {
  const match = DOC_HREF.exec((href ?? '').trim());
  return match ? match[1] : null;
}

function hasEntry(id: string): boolean {
  const data = state.index.data || {};
  return Boolean(data[id] || Object.values(data).some((entry) => entry._id === id || entry.id === id));
}

function reportOpenFailure(reason: unknown) {
  const detail = reason instanceof Error ? reason.message : String(reason);
  showActionError({ message: `Cannot open document: ${detail}` });
}

/** Rust validates the id; the document list must hold the entry before the shared open event. */
export async function openDocById(id: string): Promise<void> {
  try {
    const note = await resolveDocForOpen(id);
    if (!hasEntry(id)) {
      if (note.workspace && note.workspace !== state.workspaces.current) await showWorkspace(note.workspace);
      else await refreshNotesIndex();
    }
    if (!hasEntry(id)) throw new Error('not in the document list yet');
    document.dispatchEvent(
      new CustomEvent('cta:open-entry', { detail: { id: note.id || id } }),
    );
  } catch (err) {
    reportOpenFailure(err);
  }
}

/** Delegated click for rendered chat markdown. Only `doc:` links are intercepted. */
export function onDocLinkClick(event: {
  target: EventTarget | null;
  preventDefault(): void;
}): void {
  const target = event.target;
  if (!(target instanceof Element)) return;
  const anchor = target.closest('a[href^="doc:"]');
  if (!anchor) return;
  event.preventDefault();
  const id = docIdFromHref(anchor.getAttribute('href'));
  if (!id) {
    reportOpenFailure('invalid document link');
    return;
  }
  void openDocById(id);
}
