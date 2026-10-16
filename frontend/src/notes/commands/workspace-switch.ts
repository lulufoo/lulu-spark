import * as api from '../../host/api.ts';
import { notifyState, state } from '../state/host.ts';
import { applyListFilters, selectDate } from './sidebar.ts';
import { getCurrentWorkspace, storeWorkspaceId } from './workspace.ts';

/** Show another workspace: the page reloads its index and project titles. */
export function switchWorkspace(id: string) {
  if (!id || id === state.workspaces.current) return;
  state.workspaces.current = id;
  storeWorkspaceId(id);
  state.ui.activeTopic = null;
  state.ui.activeDate = null;
  document.dispatchEvent(new CustomEvent('cta:reload'));
}

/** Switch the current workspace between directory and timeline; the choice is stored on the workspace. */
export async function changeViewMode(mode: api.ViewMode) {
  const workspace = getCurrentWorkspace();
  if (!workspace || workspace.view_mode === mode) return;
  const data = (await api.setWorkspaceViewMode(workspace.id, mode)) as { error?: string };
  if (data?.error) throw new Error(data.error);
  state.workspaces.list = state.workspaces.list.map((w) => (w.id === workspace.id ? { ...w, view_mode: mode } : w));
  applyListFilters();
  const first = state.index.filteredGroups[0];
  if (first) selectDate(first.date);
  else notifyState();
}

/** Show another workspace and wait until its index is loaded. */
export function showWorkspace(id: string): Promise<void> {
  if (!id || id === state.workspaces.current) return Promise.resolve();
  return new Promise((resolve) => {
    document.addEventListener('cta:index-loaded', () => resolve(), { once: true });
    switchWorkspace(id);
  });
}

/** Switch to the workspace that holds document `id`; false when it is already shown or unknown. */
export async function showWorkspaceOf(id: string): Promise<boolean> {
  try {
    const note = await api.resolveNoteForOpen(id);
    if (!note.workspace || note.workspace === state.workspaces.current) return false;
    await showWorkspace(note.workspace);
    return true;
  } catch {
    return false;
  }
}
