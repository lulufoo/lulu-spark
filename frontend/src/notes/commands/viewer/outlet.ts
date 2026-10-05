import { notifyState, state } from '../../state/host.ts';
import { formatDate } from '../../../shared/utils.ts';
import type { HostNoteGroup, HostOutletMode } from '../../../host/snapshot-types.ts';
import type { NoteEntry } from '../../state/types.ts';

export function notesDateHeadingText(
  dateStr: string | null | undefined,
  group: { entries?: unknown[] } | null | undefined,
) {
  const date = String(dateStr || '').slice(0, 8);
  if (!/^\d{8}$/.test(date)) return '';
  const datePart = formatDate(date).full;
  const count = group?.entries?.length;
  return count != null ? `${datePart}  ·  ${count} items` : datePart;
}

export function findNotesDateGroup(dateStr: string | null | undefined): HostNoteGroup | null {
  const date = String(dateStr || '').slice(0, 8);
  if (!/^\d{8}$/.test(date)) return null;
  return (
    state.index.filteredGroups?.find((g) => g.date === date) ||
    state.index.groupedByDate?.find((g) => g.date === date) ||
    null
  );
}

export function resolveNotesListDate(
  activeDate?: string | null,
  routeDate?: string | null,
) {
  const picks = [
    activeDate,
    routeDate,
    state.index.filteredGroups[0]?.date,
    state.index.groupedByDate[0]?.date,
  ];
  for (const pick of picks) {
    const date = String(pick || '').slice(0, 8);
    if (!/^\d{8}$/.test(date)) continue;
    const group = findNotesDateGroup(date);
    if (group) return { date, group };
  }
  return { date: '', group: null };
}

export function setNotePanelTitle(entryOrDate?: string | NoteEntry | null) {
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
  const group = findNotesDateGroup(dateStr);
  const text = notesDateHeadingText(dateStr, group);
  state.viewer.panelTitle = text;
  if (titleEl) {
    titleEl.textContent = text;
    titleEl.removeAttribute('title');
  }
  notifyState();
}

/** Reveal note outlet chrome. React also reads outletMode; keep id writes for leftover tests. */
export function showNoteOutlet(mode?: HostOutletMode) {
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
  const dateHeading = document.getElementById('date-heading');
  if (dateHeading) {
    dateHeading.hidden = false;
    dateHeading.style.display = '';
  }
  const docList = document.getElementById('doc-list');
  if (docList) docList.style.display = '';
  notifyState();
}
