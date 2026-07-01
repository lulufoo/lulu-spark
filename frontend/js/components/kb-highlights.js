import { state } from '../state.js'
import * as api from '../api.js'
import { nowTs } from '../utils.js'
import { wrapNthMatch, getOccurrenceIndex } from './highlight-utils.js'

const BODY_ID = 'kb-md-body';
const COMMENTS_BAR_ID = 'kb-md-comments-bar';
const BTN_ID = 'kb-highlight-add-btn';

/** @type {HTMLElement | null} */
let _kbHighlightRoot = null;
/** @type {HTMLButtonElement | null} */
let _kbHighlightBtn = null;
/** @type {Array<() => void>} */
const _kbHighlightCleanups = [];

function kbHighlightBodyEl() {
  if (_kbHighlightRoot) {
    return _kbHighlightRoot.querySelector('.kb-reader-body');
  }
  return document.getElementById(BODY_ID);
}

function kbCommentsBarSelector() {
  return _kbHighlightRoot ? '.md-comments-bar' : `#${COMMENTS_BAR_ID}`;
}

function kbHighlightBtnEl() {
  return _kbHighlightBtn ?? document.getElementById(BTN_ID);
}

function resolveKbHighlightBtn() {
  const nodes = document.querySelectorAll(`#${BTN_ID}`);
  if (nodes.length > 1) {
    for (let i = 1; i < nodes.length; i += 1) nodes[i].remove();
  }
  let btn = document.getElementById(BTN_ID);
  if (!btn) {
    btn = document.createElement('button');
    btn.id = BTN_ID;
    btn.className = 'viewer-highlight-btn highlight-add-btn';
    btn.style.cssText = 'display:none;position:fixed;z-index:9999;';
    btn.textContent = '高亮';
    document.body.appendChild(btn);
  }
  return btn;
}

// ── applyKbHighlights ──────────────────────────────────────────────────────
export function applyKbHighlights(annotation) {
  const container = kbHighlightBodyEl();
  if (!container) return;
  const highlights = annotation?.highlights || [];
  for (const h of highlights) {
    wrapNthMatch(container, h.text, h.occurrence ?? 0, h.id, id => deleteKbHighlight(id), kbCommentsBarSelector());
  }
}

// ── reapplyKbHighlights ────────────────────────────────────────────────────
export function reapplyKbHighlights() {
  const container = kbHighlightBodyEl();
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

// ── initKbHighlightUI / cleanupKbHighlightUI ─────────────────────────────────

let _kbPendingText = null;
let _kbPendingOccurrence = 0;

function hideKbBtn() {
  const btn = kbHighlightBtnEl();
  if (btn) btn.style.display = 'none';
  _kbPendingText = null;
}

export function cleanupKbHighlightUI() {
  for (const fn of _kbHighlightCleanups) fn();
  _kbHighlightCleanups.length = 0;
  hideKbBtn();
  _kbHighlightRoot = null;
  _kbHighlightBtn = null;
}

export function initKbHighlightUI(container) {
  if (!container) {
    const legacyBody = document.getElementById(BODY_ID);
    container = legacyBody?.closest('.kb-reader') ?? legacyBody?.parentElement;
    if (!container) return;
  }

  cleanupKbHighlightUI();
  _kbHighlightRoot = container;

  const btn = resolveKbHighlightBtn();
  _kbHighlightBtn = btn;

  const body = kbHighlightBodyEl();
  if (!body) return;

  const onDocMouseDown = (e) => {
    if (e.target !== btn) hideKbBtn();
  };
  document.addEventListener('mousedown', onDocMouseDown);
  _kbHighlightCleanups.push(() => document.removeEventListener('mousedown', onDocMouseDown));

  const onBodyMouseUp = () => {
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed) { hideKbBtn(); return; }
    const text = sel.toString().trim();
    if (!text) { hideKbBtn(); return; }

    const editArea = container.querySelector('.kb-reader-edit-area')
      ?? document.getElementById('kb-md-edit-area');
    if (editArea && editArea.style.display !== 'none') return;

    const commentsBar = body.querySelector('.doc-comments-bar')
      ?? container.querySelector('.md-comments-bar')
      ?? document.getElementById(COMMENTS_BAR_ID);
    if (commentsBar && commentsBar.contains(sel.anchorNode)) { hideKbBtn(); return; }

    _kbPendingOccurrence = getOccurrenceIndex(body, sel, text, kbCommentsBarSelector());
    _kbPendingText = text;

    const range = sel.getRangeAt(0);
    const rect = range.getBoundingClientRect();
    btn.style.top = `${rect.top - 36}px`;
    btn.style.left = `${rect.left + rect.width / 2 - 30}px`;
    btn.style.display = 'block';
  };
  body.addEventListener('mouseup', onBodyMouseUp);
  _kbHighlightCleanups.push(() => body.removeEventListener('mouseup', onBodyMouseUp));

  const onBtnMouseDown = (e) => e.preventDefault();
  btn.addEventListener('mousedown', onBtnMouseDown);
  _kbHighlightCleanups.push(() => btn.removeEventListener('mousedown', onBtnMouseDown));

  const onBtnClick = async () => {
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
      wrapNthMatch(kbHighlightBodyEl(), text, occurrence, data.id, id => deleteKbHighlight(id), kbCommentsBarSelector());
      document.dispatchEvent(new CustomEvent('kb:dirty', { detail: { msg: 'chore: update annotations' } }));
    } catch (e) {
      alert(`高亮失败：${e.message}`);
    }
  };
  btn.addEventListener('click', onBtnClick);
  _kbHighlightCleanups.push(() => btn.removeEventListener('click', onBtnClick));
}
