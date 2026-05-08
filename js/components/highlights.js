import { state } from '../state.js'
import * as api from '../api.js'
import { nowTs } from '../utils.js'

// ── buildFlatText ──────────────────────────────────────────────────────────
// Collect visible text nodes into a flat string + position map.
// Excludes nodes inside existing highlights and the comments bar.
function buildFlatText(container) {
  const commentsBar = document.getElementById('md-comments-bar');
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
function buildNormIndex(rawText) {
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

// ── applyHighlights ────────────────────────────────────────────────────────
export function applyHighlights(annotation, layer) {
  const container = document.getElementById('md-body');
  if (!container) return;
  const highlights = annotation?.[layer]?.highlights || [];
  if (!highlights.length) return;
  for (const h of highlights) {
    wrapNthMatch(container, h.text, h.occurrence ?? 0, h.id);
  }
}

// ── wrapNthMatch ───────────────────────────────────────────────────────────
// Wrap the `occurrence`-th (0-based) normalized match of `text` with <mark>(s).
// Uses segment-level splitText — never calls Range.surroundContents, so it
// works even when the selection crosses inline/block element boundaries.
function wrapNthMatch(container, text, occurrence, id) {
  if (!text) return;
  const { rawText, segments } = buildFlatText(container);
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

  // Build the delete button (appended to the last mark fragment)
  function makeDelBtn() {
    const b = document.createElement('button');
    b.className = 'highlight-del-btn';
    b.title = '取消高亮';
    b.textContent = '×';
    b.addEventListener('click', e => { e.preventDefault(); e.stopPropagation(); deleteHighlight(id); });
    return b;
  }

  // Collect segments that overlap [rawStart, rawEnd)
  const overlapping = segments.filter(s => s.start < rawEnd && s.end > rawStart);
  if (!overlapping.length) return;

  // Wrap each overlapping text-node segment individually (splitText only, never
  // Range.surroundContents).  Purely-whitespace fragments (e.g. the '\n' text
  // nodes that marked.js emits between </p> and <p>) are skipped — wrapping them
  // in <mark> would inject an inline element between two block boxes, causing a
  // visible blank line in the layout.
  let lastMark = null;
  for (let i = 0; i < overlapping.length; i++) {
    const seg = overlapping[i];
    const lo = Math.max(rawStart, seg.start) - seg.start; // offset inside this text node
    const hi = Math.min(rawEnd,   seg.end)   - seg.start;

    // Skip purely-whitespace fragments
    if (/^\s*$/.test(seg.node.nodeValue.slice(lo, hi))) continue;

    let targetNode = seg.node;

    // Trim trailing part first (split at hi)
    if (hi < targetNode.nodeValue.length) {
      targetNode.splitText(hi); // targetNode is now [0, hi)
    }
    // Trim leading part (split at lo)
    if (lo > 0) {
      targetNode = targetNode.splitText(lo); // targetNode is now [lo, hi)
    }

    const mark = document.createElement('mark');
    mark.className = 'doc-highlight';
    mark.dataset.hid = id;
    targetNode.parentNode.insertBefore(mark, targetNode);
    mark.appendChild(targetNode);
    lastMark = mark;
  }

  // Attach the delete button to the last non-whitespace fragment
  if (lastMark) lastMark.appendChild(makeDelBtn());
}

// ── getOccurrenceIndex ─────────────────────────────────────────────────────
// Count how many times (normalized) text appears before the selection start.
function getOccurrenceIndex(container, selection, text) {
  const { rawText, segments } = buildFlatText(container);
  const range = selection.getRangeAt(0);

  // startContainer may be an element node (e.g. <li>); walk into its first text node
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
    // fallback: if we never find startNode, selRawStart stays 0 (occurrence 0)
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

// ── deleteHighlight ────────────────────────────────────────────────────────
async function deleteHighlight(id) {
  const { entry, layer, annotation } = state.viewer;
  if (!entry) return;
  try {
    const data = await api.updateHighlight(entry.common_path, layer, { id }, nowTs());
    if (!data.ok) throw new Error(data.error || 'failed');
    const layerData = annotation?.[layer];
    if (layerData?.highlights) {
      layerData.highlights = layerData.highlights.filter(h => h.id !== id);
      if (!layerData.highlights.length) delete layerData.highlights;
    }
    reapplyHighlights();
  } catch (e) {
    alert(`取消高亮失败：${e.message}`);
  }
}

// ── reapplyHighlights ──────────────────────────────────────────────────────
function reapplyHighlights() {
  const container = document.getElementById('md-body');
  // A single logical highlight may span multiple <mark> fragments (cross-element);
  // unwrap all of them, then normalize to merge adjacent text nodes.
  container.querySelectorAll('mark.doc-highlight').forEach(mark => {
    mark.querySelectorAll('.highlight-del-btn').forEach(b => b.remove());
    mark.replaceWith(...Array.from(mark.childNodes));
  });
  container.normalize();
  applyHighlights(state.viewer.annotation, state.viewer.layer);
}

// ── Floating "高亮" button ─────────────────────────────────────────────────

let _pendingText = null;
let _pendingOccurrence = 0;

export function initHighlightUI() {
  const btn = document.getElementById('highlight-add-btn');
  if (!btn) return;

  // Hide button on any click outside
  document.addEventListener('mousedown', e => {
    if (e.target !== btn) hideBtn();
  });

  // Detect text selection inside md-body
  document.getElementById('md-body').addEventListener('mouseup', () => {
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed) { hideBtn(); return; }
    const text = sel.toString().trim();
    if (!text) { hideBtn(); return; }

    // Only show button when viewing (not editing)
    const editArea = document.getElementById('md-edit-area');
    if (editArea && editArea.style.display !== 'none') return;

    const container = document.getElementById('md-body');
    _pendingOccurrence = getOccurrenceIndex(container, sel, text);
    _pendingText = text;

    const range = sel.getRangeAt(0);
    const rect  = range.getBoundingClientRect();
    btn.style.top  = `${rect.top - 36}px`;
    btn.style.left = `${rect.left + rect.width / 2 - 30}px`;
    btn.style.display = 'block';
  });

  btn.addEventListener('mousedown', e => e.preventDefault()); // prevent deselect
  btn.addEventListener('click', async () => {
    const text = _pendingText;
    const occurrence = _pendingOccurrence;
    hideBtn();
    window.getSelection()?.removeAllRanges();
    if (!text || !state.viewer.entry) return;

    const { entry, layer, annotation } = state.viewer;
    try {
      const data = await api.updateHighlight(entry.common_path, layer, { text, occurrence }, nowTs());
      if (!data.ok) throw new Error(data.error || 'failed');
      const layerData = annotation[layer] || (annotation[layer] = {});
      (layerData.highlights || (layerData.highlights = [])).push({ id: data.id, text, occurrence, ts: nowTs() });
      wrapNthMatch(document.getElementById('md-body'), text, occurrence, data.id);
    } catch (e) {
      alert(`高亮失败：${e.message}`);
    }
  });
}

function hideBtn() {
  const btn = document.getElementById('highlight-add-btn');
  if (btn) btn.style.display = 'none';
  _pendingText = null;
}
