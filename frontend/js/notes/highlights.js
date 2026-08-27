import { state } from '../host/state.js'
import * as api from '../host/api.js'
import { nowTs } from '../utils.js'
import { buildFlatText, buildNormIndex, wrapNthMatch, getOccurrenceIndex } from '../components/highlight-utils.js'
// ── applyHighlights ────────────────────────────────────────────────────────
export function applyHighlights(annotation, layer) {
  const container = document.getElementById('md-body');
  if (!container) return;
  const highlights = annotation?.[layer]?.highlights || [];
  if (!highlights.length) return;
  for (const h of highlights) {
    wrapNthMatch(container, h.text, h.occurrence ?? 0, h.id, id => deleteHighlight(id));
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
    alert(`Failed to remove highlight：${e.message}`);
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

// ── Floating "Highlight" button ─────────────────────────────────────────────────

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

    // Don't show button when selection is inside the comments bar
    const commentsBar = document.getElementById('md-comments-bar');
    if (commentsBar && commentsBar.contains(sel.anchorNode)) { hideBtn(); return; }

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
      wrapNthMatch(document.getElementById('md-body'), text, occurrence, data.id, id => deleteHighlight(id));
    } catch (e) {
      alert(`Highlight failed：${e.message}`);
    }
  });
}

function hideBtn() {
  const btn = document.getElementById('highlight-add-btn');
  if (btn) btn.style.display = 'none';
  _pendingText = null;
}
