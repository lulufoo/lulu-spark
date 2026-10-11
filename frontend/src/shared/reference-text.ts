export type ReferenceTextKind = 'note' | 'knowledge';

export type ReferenceTextInput = {
  kind: ReferenceTextKind;
  title: string;
  id: string;
};

/**
 * Plain text that tells an outside tool (for example Cursor) what a Spark document is.
 * It carries the id only. No file path, so the tool has to go through MCP to read it.
 */
export function formatReferenceText({ kind, title, id }: ReferenceTextInput) {
  const cleanTitle = String(title ?? '').replace(/\s+/g, ' ').trim();
  return [`Lulu Spark ${kind}`, `title: ${cleanTitle}`, `id: ${String(id ?? '').trim()}`].join('\n');
}

/** Write text to the clipboard. Resolves false when the clipboard is unavailable or refuses. */
export async function writeClipboardText(text: string) {
  if (!text || !navigator.clipboard?.writeText) return false;
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}
