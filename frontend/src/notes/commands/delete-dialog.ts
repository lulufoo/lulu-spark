import { getEntryId, state } from '../state/host.ts';
import * as api from '../../host/api.ts';
import { deleteOpenStore } from '../state/dialog-open.ts';

export { deleteOpenStore };

export function openDeleteDialog() {
  deleteOpenStore.set(true);
  document.getElementById('delete-dialog')?.classList.add('open');
}

export function closeDeleteDialog() {
  deleteOpenStore.set(false);
  document.getElementById('delete-dialog')?.classList.remove('open');
}

export async function confirmDeleteDocument() {
  const entryId = getEntryId(state.viewer.entry);
  const data = await api.deleteEntry(entryId);
  if (data.error) throw new Error(data.error);
  const date = state.ui.activeDate;
  closeDeleteDialog();
  if (date) {
    window.location.replace(`#/workbench?date=${encodeURIComponent(date)}`);
  } else {
    window.location.replace('#/workbench');
  }
  document.dispatchEvent(new CustomEvent('cta:reload'));
}
