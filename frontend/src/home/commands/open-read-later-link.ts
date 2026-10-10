import { loadReadLaterEntries, openExternalUrl } from '../../read-later/commands/list.ts';
import { showActionError } from './hub.ts';

/** Chat links that open a read-later entry: `[title](read-later:<32-hex id>)`. */
const READ_LATER_HREF = /^read-later:([0-9a-f]{32})$/;

export function readLaterIdFromHref(href: string | null | undefined): string | null {
  const match = READ_LATER_HREF.exec((href ?? '').trim());
  return match ? match[1] : null;
}

function reportOpenFailure(reason: unknown) {
  const detail = reason instanceof Error ? reason.message : String(reason);
  showActionError({ message: `Cannot open read-later: ${detail}` });
}

/** List lookup; then the same opener the read-later list uses. Does not mark read. */
export async function openReadLaterById(id: string): Promise<void> {
  try {
    const entries = await loadReadLaterEntries();
    const entry = Array.isArray(entries)
      ? entries.find((item) => item && item.id === id)
      : undefined;
    const url = typeof entry?.url === 'string' ? entry.url.trim() : '';
    if (!url) throw new Error('Not found');
    await openExternalUrl(url);
  } catch (err) {
    reportOpenFailure(err);
  }
}

/** Delegated click for rendered chat markdown. Only `read-later:` links are intercepted. */
export function onReadLaterLinkClick(event: {
  target: EventTarget | null;
  preventDefault(): void;
}): void {
  const target = event.target;
  if (!(target instanceof Element)) return;
  const anchor = target.closest('a[href^="read-later:"]');
  if (!anchor) return;
  event.preventDefault();
  const id = readLaterIdFromHref(anchor.getAttribute('href'));
  if (!id) {
    reportOpenFailure('invalid read-later link');
    return;
  }
  void openReadLaterById(id);
}
