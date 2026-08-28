// @ts-nocheck — DOM wiring stays unchecked like checkJs:false.
import { state } from '../../host/state.ts';
import { formatDate } from '../../shared/utils.ts';

export function setNotePanelTitle(entryOrDate) {
  const titleEl = document.getElementById('md-panel-title');
  if (!titleEl) return;
  const dateStr = typeof entryOrDate === 'string'
    ? entryOrDate
    : String(entryOrDate?.created_at || state.ui.activeDate || '').slice(0, 8);
  if (!/^\d{8}$/.test(dateStr)) {
    titleEl.textContent = '';
    titleEl.removeAttribute('title');
    return;
  }
  const group =
    state.index.filteredGroups?.find(g => g.date === dateStr) ||
    state.index.groupedByDate?.find(g => g.date === dateStr);
  const count = group?.entries?.length;
  const datePart = formatDate(dateStr).full;
  titleEl.textContent = count != null
    ? `${datePart}  ·  ${count} items`
    : datePart;
  titleEl.removeAttribute('title');
}

/** Reveal note outlet chrome (embedded #note-outlet; no Dialog display semantics). */
export function showNoteOutlet(mode) {
  const outlet = document.getElementById('note-outlet');
  if (outlet) {
    outlet.hidden = false;
    if (mode) outlet.dataset.wbMode = mode;
  }
  const dateHeading = document.getElementById('date-heading');
  if (dateHeading) dateHeading.style.display = 'none';
  const docList = document.getElementById('doc-list');
  if (docList) docList.style.display = 'none';
  const msg = document.getElementById('note-outlet-message');
  if (msg && mode !== 'safe-empty') {
    msg.hidden = true;
    msg.textContent = '';
  }
  const panel = document.getElementById('md-panel');
  if (panel && mode !== 'safe-empty') panel.hidden = false;
}

export function hideNoteOutlet() {
  const outlet = document.getElementById('note-outlet');
  if (outlet) {
    outlet.hidden = true;
    outlet.dataset.wbMode = '';
    delete outlet.dataset.note;
    delete outlet.dataset.layer;
  }
}
