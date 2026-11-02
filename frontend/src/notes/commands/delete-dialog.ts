import { refreshAuthSession } from '../../auth/refresh.ts';
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
  // The host removes the cloud copy first, with the session it reads from the vault, and does not
  // refresh it. Refresh here so a session about to expire does not fail the delete. The outcome is
  // not needed: not signed in deletes only here, and a session that cannot be refreshed is
  // answered by the host (the delete fails and the document stays).
  await refreshAuthSession().catch(() => {});
  const data = await api.deleteEntry(entryId);
  if (data.error) throw new Error(data.error);
  const date = state.ui.activeDate;
  closeDeleteDialog();
  if (date) {
    window.location.replace(`#/spark?date=${encodeURIComponent(date)}`);
  } else {
    window.location.replace('#/spark');
  }
  document.dispatchEvent(new CustomEvent('cta:reload'));
}
