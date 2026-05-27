/**
 * Markdown helpers for comment / note editors (paste + render).
 */

/** GFM table separator rows: normalize em/en dash to hyphen so marked parses tables. */
export function normalizeTableSeparators(md) {
  if (!md) return md;
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

/** Prefer plain text on paste; Turndown only when plain is empty (rich HTML from web). */
export function pasteTextFromClipboard(clipboardData) {
  const plain = clipboardData.getData('text/plain');
  if (plain && plain.trim()) {
    return plain;
  }
  const html = clipboardData.getData('text/html');
  if (html && typeof TurndownService !== 'undefined') {
    return new TurndownService({ headingStyle: 'atx', bulletListMarker: '-' }).turndown(html);
  }
  return plain || '';
}

export function renderCommentMarkdown(text) {
  const normalized = normalizeTableSeparators(text || '');
  if (typeof marked !== 'undefined') {
    return marked.parse(normalized);
  }
  return `<pre>${normalized}</pre>`;
}
