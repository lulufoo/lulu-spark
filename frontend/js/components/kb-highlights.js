import { state } from '../state.js'
import * as api from '../api.js'
import { nowTs } from '../utils.js'
import { wrapNthMatch, getOccurrenceIndex } from './highlight-utils.js'

const BODY_ID = 'kb-md-body';
const COMMENTS_BAR_ID = 'kb-md-comments-bar';
const BTN_ID = 'kb-highlight-add-btn';

// ── applyKbHighlights ──────────────────────────────────────────────────────
export function applyKbHighlights(annotation) {
  const container = document.getElementById(BODY_ID);
  if (!container) return;
  const highlights = annotation?.highlights || [];
  for (const h of highlights) {
    wrapNthMatch(container, h.text, h.occurrence ?? 0, h.id, id => deleteKbHighlight(id), COMMENTS_BAR_ID);
  }
}

// ── reapplyKbHighlights ────────────────────────────────────────────────────
export function reapplyKbHighlights() {
  const container = document.getElementById(BODY_ID);
  if (!container) return;
  container.querySelectorAll('mark.doc-highlight').forEach(mark => {
    mark.querySelectorAll('.highlight-del-btn').forEach(b => b.remove());
    mark.replaceWith(...Array.from(mark.childNodes));
  });
  container.normalize();
  applyKbHighlights(state.viewer.annotation);
}

// ── deleteKbHighlight ──────────────────────────────────────────────────────
export async function deleteKbHighlight(id) {
  const { kbRepo, kbPath, annotation } = state.viewer;
  if (!kbRepo || !kbPath) return;
  try {
    const data = await api.updateKbHighlight(kbRepo, kbPath, { id }, nowTs());
    if (!data.ok) throw new Error(data.error || 'failed');
    if (annotation?.highlights) {
      annotation.highlights = annotation.highlights.filter(h => h.id !== id);
    }
    reapplyKbHighlights();
    document.dispatchEvent(new CustomEvent('kb:dirty', { detail: { msg: 'chore: update annotations' } }));
  } catch (e) {
    alert(`取消高亮失败：${e.message}`);
  }
}

// ── initKbHighlightUI ──────────────────────────────────────────────────────

let _kbPendingText = null;
let _kbPendingOccurrence = 0;

export function initKbHighlightUI() {
  const btn = document.getElementById(BTN_ID);
  const body = document.getElementById(BODY_ID);
  if (!btn || !body) return;

  document.addEventListener('mousedown', e => {
    if (e.target !== btn) hideKbBtn();
  });

  body.addEventListener('mouseup', () => {
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed) { hideKbBtn(); return; }
    const text = sel.toString().trim();
    if (!text) { hideKbBtn(); return; }

    const editArea = document.getElementById('kb-md-edit-area');
    if (editArea && editArea.style.display !== 'none') return;

    const commentsBar = document.getElementById(COMMENTS_BAR_ID);
    if (commentsBar && commentsBar.contains(sel.anchorNode)) { hideKbBtn(); return; }

    _kbPendingOccurrence = getOccurrenceIndex(body, sel, text, COMMENTS_BAR_ID);
    _kbPendingText = text;

    const range = sel.getRangeAt(0);
    const rect = range.getBoundingClientRect();
    btn.style.top = `${rect.top - 36}px`;
    btn.style.left = `${rect.left + rect.width / 2 - 30}px`;
    btn.style.display = 'block';
  });

  btn.addEventListener('mousedown', e => e.preventDefault());
  btn.addEventListener('click', async () => {
    const text = _kbPendingText;
    const occurrence = _kbPendingOccurrence;
    hideKbBtn();
    window.getSelection()?.removeAllRanges();
    if (!text || !state.viewer.kbRepo) return;

    const { kbRepo, kbPath, annotation } = state.viewer;
    try {
      const data = await api.updateKbHighlight(kbRepo, kbPath, { text, occurrence }, nowTs());
      if (!data.ok) throw new Error(data.error || 'failed');
      if (!annotation.highlights) annotation.highlights = [];
      annotation.highlights.push({ id: data.id, text, occurrence, ts: nowTs() });
      wrapNthMatch(document.getElementById(BODY_ID), text, occurrence, data.id, id => deleteKbHighlight(id), COMMENTS_BAR_ID);
      document.dispatchEvent(new CustomEvent('kb:dirty', { detail: { msg: 'chore: update annotations' } }));
    } catch (e) {
      alert(`高亮失败：${e.message}`);
    }
  });
}

function hideKbBtn() {
  const btn = document.getElementById(BTN_ID);
  if (btn) btn.style.display = 'none';
  _kbPendingText = null;
}
