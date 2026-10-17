import { onDocLinkClick } from './open-doc-link.ts';
import { onReadLaterLinkClick } from './open-read-later-link.ts';

type LinkClickEvent = { target: EventTarget | null; preventDefault(): void };

/** One delegated click for chat markdown: each handler only reacts to its own link scheme. */
export function onChatLinkClick(event: LinkClickEvent): void {
  onDocLinkClick(event);
  onReadLaterLinkClick(event);
}
