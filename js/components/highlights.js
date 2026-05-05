import { state, getEntryId } from '../state.js'
import * as api from '../api.js'
import { nowTs } from '../utils.js'

// ── applyHighlights ────────────────────────────────────────────────────────
// Walk #md-body text nodes and wrap matching highlight texts with <mark>

export function applyHighlights(annotation, layer) {
  const container = document.getElementById('md-body');
  if (!container) return;
  const highlights = annotation?.[layer]?.highlights || [];
  if (!highlights.length) return;

  for (const h of highlights) {
    wrapNthMatch(container, h.text, h.occurrence ?? 0, h.id);
  }
}

// Collect visible text nodes (skipping content already inside a .doc-highlight mark, and the comments bar)
function collectTextNodes(container) {
  const commentsBar = document.getElementById('md-comments-bar');
  const nodes = [];
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
  while ((node = walker.nextNode())) nodes.push(node);
  return nodes;
}

// Wrap the `occurrence`-th (0-based) match of `text` in the container
function wrapNthMatch(container, text, occurrence, id) {
  if (!text) return;
  const textNodes = collectTextNodes(container);
  let found = 0;

  for (const tn of textNodes) {
    let pos = 0;
    let idx;
    while ((idx = tn.nodeValue.indexOf(text, pos)) !== -1) {
      if (found === occurrence) {
        // This is the target occurrence — wrap it
        const before = tn.nodeValue.slice(0, idx);
        const after  = tn.nodeValue.slice(idx + text.length);

        const mark = document.createElement('mark');
        mark.className = 'doc-highlight';
        mark.dataset.hid = id;

        const textNode = document.createTextNode(text);
        mark.appendChild(textNode);

        const delBtn = document.createElement('button');
        delBtn.className = 'highlight-del-btn';
        delBtn.title = '取消高亮';
        delBtn.textContent = '×';
        delBtn.addEventListener('click', e => {
          e.preventDefault();
          e.stopPropagation();
          deleteHighlight(id);
        });
        mark.appendChild(delBtn);

        const parent = tn.parentNode;
        if (before) parent.insertBefore(document.createTextNode(before), tn);
        parent.insertBefore(mark, tn);
        if (after) parent.insertBefore(document.createTextNode(after), tn);
        parent.removeChild(tn);
        return;
      }
      found++;
      pos = idx + text.length;
    }
  }
}

// Calculate which occurrence (0-based) the current selection represents,
// counting only within main content (excluding the comments bar)
function getOccurrenceIndex(container, selection, text) {
  const commentsBar = document.getElementById('md-comments-bar');
  const range = selection.getRangeAt(0);
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

  let count = 0;
  let node;
  while ((node = walker.nextNode())) {
    const nodeRange = document.createRange();
    nodeRange.selectNodeContents(node);
    // If node ends before selection starts, count all occurrences in it
    if (range.compareBoundaryPoints(Range.END_TO_START, nodeRange) > 0) {
      let pos = 0, idx;
      while ((idx = node.nodeValue.indexOf(text, pos)) !== -1) { count++; pos = idx + text.length; }
    } else {
      // Node contains or follows the selection start — only count up to startOffset
      if (node === range.startContainer) {
        const partial = node.nodeValue.slice(0, range.startOffset);
        let pos = 0, idx;
        while ((idx = partial.indexOf(text, pos)) !== -1) { count++; pos = idx + text.length; }
      }
      break;
    }
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

// Re-apply highlights without re-fetching content (works on current rendered DOM)
function reapplyHighlights() {
  const container = document.getElementById('md-body');
  container.querySelectorAll('mark.doc-highlight').forEach(mark => {
    // Get just the text content (first text node child, before the × button)
    const textContent = Array.from(mark.childNodes)
      .filter(n => n.nodeType === Node.TEXT_NODE)
      .map(n => n.nodeValue)
      .join('');
    mark.replaceWith(document.createTextNode(textContent));
  });
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
