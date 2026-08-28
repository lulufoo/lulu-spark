// @ts-nocheck — outlet mode lives on host/state; leftover callers still use these names.
import { notifyState, state } from '../../state/host.ts';
import { formatDate } from '../../../shared/utils.ts';

export function setNotePanelTitle(entryOrDate) {
  const titleEl = document.getElementById('md-panel-title');
  const dateStr = typeof entryOrDate === 'string'
    ? entryOrDate
    : String(entryOrDate?.created_at || state.ui.activeDate || '').slice(0, 8);
  if (!/^\d{8}$/.test(dateStr)) {
    if (titleEl) {
      titleEl.textContent = '';
      titleEl.removeAttribute('title');
    }
    return;
  }
  const group =
    state.index.filteredGroups?.find(g => g.date === dateStr) ||
    state.index.groupedByDate?.find(g => g.date === dateStr);
  const count = group?.entries?.length;
  const datePart = formatDate(dateStr).full;
  const text = count != null
    ? `${datePart}  ·  ${count} items`
    : datePart;
  state.viewer.panelTitle = text;
  if (titleEl) {
    titleEl.textContent = text;
    titleEl.removeAttribute('title');
  }
  notifyState();
}

/** Reveal note outlet chrome. React also reads outletMode; keep id writes for leftover tests. */
export function showNoteOutlet(mode) {
  state.viewer.outletMode = mode || 'open';
  if (mode !== 'safe-empty') state.viewer.outletMessage = '';
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
  notifyState();
}

export function hideNoteOutlet() {
  state.viewer.outletMode = '';
  state.viewer.outletMessage = '';
  const outlet = document.getElementById('note-outlet');
  if (outlet) {
    outlet.hidden = true;
    outlet.dataset.wbMode = '';
    delete outlet.dataset.note;
    delete outlet.dataset.layer;
  }
  notifyState();
}
