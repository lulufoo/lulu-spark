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
    wrapFirstMatch(container, h.text, h.id);
  }
}

function wrapFirstMatch(container, text, id) {
  if (!text) return;
  // Collect text nodes (skip nodes already inside a mark)
  const textNodes = [];
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      let p = node.parentNode;
      while (p && p !== container) {
        if (p.tagName === 'MARK') return NodeFilter.FILTER_REJECT;
        p = p.parentNode;
      }
      return NodeFilter.FILTER_ACCEPT;
    }
  });
  let node;
  while ((node = walker.nextNode())) textNodes.push(node);

  for (const tn of textNodes) {
    const idx = tn.nodeValue.indexOf(text);
    if (idx === -1) continue;

    const before = tn.nodeValue.slice(0, idx);
    const after  = tn.nodeValue.slice(idx + text.length);

    const mark = document.createElement('mark');
    mark.className = 'doc-highlight';
    mark.dataset.hid = id;
    mark.textContent = text;

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
    return; // first match only
  }
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
  // Remove all existing mark wrappers, restoring plain text
  const container = document.getElementById('md-body');
  container.querySelectorAll('mark.doc-highlight').forEach(mark => {
    const text = document.createTextNode(mark.textContent.replace(/×$/, '').trimEnd());
    // textContent includes the × button text; get just the highlight text
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

    const range = sel.getRangeAt(0);
    const rect  = range.getBoundingClientRect();
    _pendingText = text;

    btn.style.top  = `${rect.top - 36}px`;
    btn.style.left = `${rect.left + rect.width / 2 - 30}px`;
    btn.style.display = 'block';
  });

  btn.addEventListener('mousedown', e => e.preventDefault()); // prevent deselect
  btn.addEventListener('click', async () => {
    const text = _pendingText;
    hideBtn();
    window.getSelection()?.removeAllRanges();
    if (!text || !state.viewer.entry) return;

    const { entry, layer, annotation } = state.viewer;
    try {
      const data = await api.updateHighlight(entry.common_path, layer, { text }, nowTs());
      if (!data.ok) throw new Error(data.error || 'failed');
      const layerData = annotation[layer] || (annotation[layer] = {});
      (layerData.highlights || (layerData.highlights = [])).push({ id: data.id, text, ts: nowTs() });
      wrapFirstMatch(document.getElementById('md-body'), text, data.id);
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
