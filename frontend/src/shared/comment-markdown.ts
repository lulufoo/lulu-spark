/**
 * Markdown helpers for comment / note editors (paste + render).
 */

type ClipboardLike = { getData: (type: string) => string };

/** GFM table separator rows: normalize em/en dash to hyphen so marked parses tables. */
export function normalizeTableSeparators(md: string | null | undefined): string {
  if (!md) return md ?? '';
  return md.replace(/^\|([^\n]*)\|\s*$/gm, (line) => {
    const cells = line.slice(1, -1).split('|');
    const isSeparator = cells.length > 0 && cells.every(c => /^[\s:\-—–]+$/.test(c));
    if (!isSeparator) return line;
    return '|' + cells.map(c => {
      if (/^[\s:—–]+$/.test(c) && /[—–]/.test(c)) {
        return c.replace(/[—–]+/g, '---');
      }
      return c.replace(/[—–]/g, '-');
    }).join('|') + '|';
  });
}

/** Normalize note markdown for storage / editor display (not just render). */
export function prepareCommentMarkdown(text: string | null | undefined): string {
  return normalizeTableSeparators(text || '');
}

/** Prefer plain text on paste; Turndown only when plain is empty (rich HTML from web). */
export function pasteTextFromClipboard(clipboardData: ClipboardLike | null | undefined): string {
  if (!clipboardData) return prepareCommentMarkdown('');
  const plain = clipboardData.getData('text/plain');
  if (plain && plain.trim()) {
    return prepareCommentMarkdown(plain);
  }
  const html = clipboardData.getData('text/html');
  if (html && typeof TurndownService !== 'undefined') {
    return prepareCommentMarkdown(
      new TurndownService({ headingStyle: 'atx', bulletListMarker: '-' }).turndown(html)
    );
  }
  return prepareCommentMarkdown(plain || '');
}

/** Insert plain text without execCommand (avoids macOS smart-dash rewriting --- → —). */
export function insertCommentPlainText(el: HTMLElement | null | undefined, text: string | null | undefined) {
  if (!el || !text) return;
  const normalized = prepareCommentMarkdown(text);
  const sel = window.getSelection();
  if (sel && sel.rangeCount > 0) {
    const range = sel.getRangeAt(0);
    range.deleteContents();
    const node = document.createTextNode(normalized);
    range.insertNode(node);
    range.setStartAfter(node);
    range.collapse(true);
    sel.removeAllRanges();
    sel.addRange(range);
  } else {
    el.appendChild(document.createTextNode(normalized));
  }
  const fixed = prepareCommentMarkdown(el.innerText);
  if (fixed !== el.innerText) {
    el.innerText = fixed;
    const range = document.createRange();
    range.selectNodeContents(el);
    range.collapse(false);
    sel?.removeAllRanges();
    sel?.addRange(range);
  }
}

export function pasteIntoCommentEditor(
  el: HTMLElement | null | undefined,
  clipboardData: ClipboardLike | null | undefined,
) {
  if (!clipboardData) return;
  insertCommentPlainText(el, pasteTextFromClipboard(clipboardData));
}

export function renderCommentMarkdown(text: string | null | undefined): string {
  const normalized = prepareCommentMarkdown(text);
  if (typeof marked !== 'undefined') {
    return marked.parse(normalized);
  }
  return `<pre>${normalized}</pre>`;
}
