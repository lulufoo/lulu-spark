import { resolveKnowledgeForOpen } from '../../host/api.ts';
import { showActionError } from './hub.ts';

/** Chat links that open a knowledge document: `[title](knowledge:<hex id>)`. Written by the model.
 * The id length is not checked here; Rust decides whether the id exists. */
const KNOWLEDGE_HREF = /^knowledge:([0-9a-f]+)$/;

export function knowledgeIdFromHref(href: string | null | undefined): string | null {
  const match = KNOWLEDGE_HREF.exec((href ?? '').trim());
  return match ? match[1] : null;
}

function reportOpenFailure(reason: unknown) {
  const detail = reason instanceof Error ? reason.message : String(reason);
  showActionError({ message: `Cannot open knowledge document: ${detail}` });
}

/** Rust resolves the id to the knowledge page route; the page opens through the shared event. */
export async function openKnowledgeById(id: string): Promise<void> {
  try {
    const doc = await resolveKnowledgeForOpen(id);
    if (!doc.repo || !doc.path) throw new Error('missing library location');
    document.dispatchEvent(
      new CustomEvent('cta:open-kb-doc', { detail: { repo: doc.repo, path: doc.path } }),
    );
  } catch (err) {
    reportOpenFailure(err);
  }
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
    reportOpenFailure('invalid knowledge link');
    return;
  }
  void openKnowledgeById(id);
}
