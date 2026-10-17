import { showActionError } from './hub.ts';
import { openNoteById } from './open-note-link.ts';

/** Chat links that open a knowledge document: `[title](knowledge:<hex id>)`. Written by the model.
 * The id length is not checked here; Rust decides whether the id exists. */
const KNOWLEDGE_HREF = /^knowledge:((?:[a-z0-9_]+-)?[0-9a-f]+)$/;

export function knowledgeIdFromHref(href: string | null | undefined): string | null {
  const match = KNOWLEDGE_HREF.exec((href ?? '').trim());
  return match ? match[1] : null;
}

/** Knowledge documents are workspace documents now, so they open like any other document. */
export async function openKnowledgeById(id: string): Promise<void> {
  await openNoteById(id);
}

/** Delegated click for rendered chat markdown. Only `knowledge:` links are intercepted. */
export function onKnowledgeLinkClick(event: {
  target: EventTarget | null;
  preventDefault(): void;
}): void {
  const target = event.target;
  if (!(target instanceof Element)) return;
  const anchor = target.closest('a[href^="knowledge:"]');
  if (!anchor) return;
  event.preventDefault();
  const id = knowledgeIdFromHref(anchor.getAttribute('href'));
  if (!id) {
    showActionError({ message: 'Cannot open knowledge document: invalid knowledge link' });
    return;
  }
  void openKnowledgeById(id);
}
