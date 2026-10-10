import { onKnowledgeLinkClick } from './open-knowledge-link.ts';
import { onNoteLinkClick } from './open-note-link.ts';

type LinkClickEvent = { target: EventTarget | null; preventDefault(): void };

/** One delegated click for chat markdown: each handler only reacts to its own link scheme. */
export function onChatLinkClick(event: LinkClickEvent): void {
  onNoteLinkClick(event);
  onKnowledgeLinkClick(event);
}
