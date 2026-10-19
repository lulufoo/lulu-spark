import * as api from '../../../host/api.ts';
import { announceRegistryChange } from '../../../shared/registry-events.ts';
import { enterProject } from '../../../notes/commands/restore.ts';
import { state } from '../../../host/state.ts';
import { notesCategoriesStore, patchNotesCategories } from '../../state/settings/notes-categories.ts';
import { loadNotesCategories } from './notes-categories.ts';

/** Open the confirm dialog for project `id`. The built-in project cannot be deleted. */
export function askDeleteProject(id: string) {
  const snap = notesCategoriesStore.getSnapshot();
  const row = snap.rows.find((r) => r.id === id);
  if (!row || row.is_default || snap.busy) return;
  patchNotesCategories({ deleteId: id, deleteError: '' });
}

export function cancelDeleteProject() {
  const snap = notesCategoriesStore.getSnapshot();
  if (snap.busy || !snap.deleteId) return;
  patchNotesCategories({ deleteId: '', deleteError: '' });
}

/**
 * The page keeps its own copy of the documents. When the browsed workspace lost a project,
 * reload it; a page that was showing the deleted project goes to the built-in one.
 */
async function refreshPage(workspaceId: string, deletedId: string, inboxId: string) {
  if (workspaceId !== state.workspaces.current) return;
  const wasShown = state.ui.activeTopic === deletedId;
  const loaded = new Promise<void>((resolve) => {
    document.addEventListener('cta:index-loaded', () => resolve(), { once: true });
  });
  document.dispatchEvent(new CustomEvent('cta:reload'));
  await loaded;
  if (wasShown && inboxId) enterProject(inboxId);
}

/**
 * Delete the project the dialog asks about; its documents move to the built-in project. A repeat
 * click while busy does nothing. A failure stays in the dialog and leaves the list as it is.
 */
export async function confirmDeleteProject(workspaceId: string): Promise<boolean> {
  const snap = notesCategoriesStore.getSnapshot();
  const id = snap.deleteId;
  if (!id || snap.busy) return false;
  const inboxId = snap.rows.find((r) => r.is_default)?.id ?? '';
  patchNotesCategories({ busy: true, deleteError: '' });
  try {
    const result = await api.deleteProject(workspaceId, id);
    if (result?.error) throw new Error(result.error);
  } catch (e) {
    patchNotesCategories({ busy: false, deleteError: (e as Error).message || String(e) });
    return false;
  }
  patchNotesCategories({ deleteId: '', busy: false });
  await loadNotesCategories(workspaceId);
  announceRegistryChange();
  void refreshPage(workspaceId, id, inboxId).catch(() => {});
  return true;
}
