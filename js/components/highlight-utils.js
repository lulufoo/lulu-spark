// Pure highlight algorithm utilities — no DOM IDs hardcoded, no state dependency.

// ── buildFlatText ──────────────────────────────────────────────────────────
// Collect visible text nodes into a flat string + position map.
// Excludes nodes inside existing highlights and the comments bar (by ID).
export function buildFlatText(container, commentsBarId = 'md-comments-bar') {
  const commentsBar = commentsBarId ? document.getElementById(commentsBarId) : null;
  const segments = [];
  let offset = 0;
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      if (commentsBar && commentsBar.contains(node)) return NodeFilter.FILTER_REJECT;
      let p = node.parentNode;
      while (p && p !== container) {
        if (p.classList?.contains('doc-highlight')) return NodeFilter.FILTER_REJECT;
        p = p.parentNode;
      }
      return NodeFilter.FILTER_ACCEPT;
    }
  });
  let node;
  while ((node = walker.nextNode())) {
    const len = node.nodeValue.length;
    segments.push({ node, start: offset, end: offset + len });
    offset += len;
  }
  const rawText = segments.map(s => s.node.nodeValue).join('');
  return { rawText, segments };
}

// ── buildNormIndex ─────────────────────────────────────────────────────────
// Collapse whitespace runs in rawText to a single space.
// origPositions[i] = raw index of the i-th normalized char.
// origPositions[normText.length] = rawText.length  (end sentinel).
export function buildNormIndex(rawText) {
  const chars = [];
  const origPositions = [];
  let inWS = false;
  for (let i = 0; i < rawText.length; i++) {
    if (/[\s\u00a0\u200b]/.test(rawText[i])) {
      if (!inWS) { chars.push(' '); origPositions.push(i); inWS = true; }
    } else {
      chars.push(rawText[i]); origPositions.push(i); inWS = false;
    }
  }
  origPositions.push(rawText.length);
  return { normText: chars.join(''), origPositions };
}

// ── wrapNthMatch ───────────────────────────────────────────────────────────
// Wrap the `occurrence`-th (0-based) normalized match of `text` with <mark>(s).
// onDeleteClick(id) is called when the delete button is clicked.
export function wrapNthMatch(container, text, occurrence, id, onDeleteClick, commentsBarId = 'md-comments-bar') {
  if (!text) return;
  const { rawText, segments } = buildFlatText(container, commentsBarId);
  const { normText, origPositions } = buildNormIndex(rawText);
  const normSearch = text.replace(/[\s\u00a0\u200b]+/g, ' ').trim();
  if (!normSearch) return;

  // Find the nth occurrence in normalized text
  let found = 0, pos = 0, normStart = -1;
  while (true) {
    const idx = normText.indexOf(normSearch, pos);
    if (idx === -1) return;
    if (found === occurrence) { normStart = idx; break; }
    found++; pos = idx + 1;
  }

  const rawStart = origPositions[normStart];
  const rawEnd   = origPositions[normStart + normSearch.length];

  function makeDelBtn() {
    const b = document.createElement('button');
    b.className = 'highlight-del-btn';
    b.title = '取消高亮';
    b.textContent = '×';
    b.addEventListener('click', e => {
      e.preventDefault();
      e.stopPropagation();
      if (onDeleteClick) onDeleteClick(id);
    });
    return b;
  }

  const overlapping = segments.filter(s => s.start < rawEnd && s.end > rawStart);
  if (!overlapping.length) return;

  let lastMark = null;
  for (let i = 0; i < overlapping.length; i++) {
    const seg = overlapping[i];
    const lo = Math.max(rawStart, seg.start) - seg.start;
    const hi = Math.min(rawEnd,   seg.end)   - seg.start;

    if (/^\s*$/.test(seg.node.nodeValue.slice(lo, hi))) continue;

    let targetNode = seg.node;

    if (hi < targetNode.nodeValue.length) {
      targetNode.splitText(hi);
    }
    if (lo > 0) {
      targetNode = targetNode.splitText(lo);
    }

    const mark = document.createElement('mark');
    mark.className = 'doc-highlight';
    mark.dataset.hid = id;
    targetNode.parentNode.insertBefore(mark, targetNode);
    mark.appendChild(targetNode);
    lastMark = mark;
  }

  if (lastMark) lastMark.appendChild(makeDelBtn());
}

// ── getOccurrenceIndex ─────────────────────────────────────────────────────
// Count how many times (normalized) text appears before the selection start.
export function getOccurrenceIndex(container, selection, text, commentsBarId = 'md-comments-bar') {
  const { rawText, segments } = buildFlatText(container, commentsBarId);
  const range = selection.getRangeAt(0);

  let startNode = range.startContainer;
  let startOffset = range.startOffset;
  if (startNode.nodeType !== Node.TEXT_NODE) {
    const child = startNode.childNodes[startOffset] || startNode.firstChild;
    if (child) { startNode = child; startOffset = 0; }
  }

  let selRawStart = 0;
  for (const seg of segments) {
    if (seg.node === startNode) {
      selRawStart = seg.start + startOffset;
      break;
    }
  }

  const { normText, origPositions } = buildNormIndex(rawText);
  const normSearch = text.replace(/[\s\u00a0\u200b]+/g, ' ').trim();
  if (!normSearch) return 0;

  let count = 0, pos = 0;
  while (true) {
    const idx = normText.indexOf(normSearch, pos);
    if (idx === -1 || origPositions[idx] >= selRawStart) break;
    count++; pos = idx + 1;
  }
  return count;
}
