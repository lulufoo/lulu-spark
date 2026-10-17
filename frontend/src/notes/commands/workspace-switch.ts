import * as api from '../../host/api.ts';
import { notifyState, state } from '../state/host.ts';
import { applyListFilters, selectDate, selectProject } from './sidebar.ts';
import { isDirectoryView, storeWorkspaceId } from './workspace.ts';

/** Show another workspace: the page reloads its index and project titles. */
export function switchWorkspace(id: string) {
  if (!id || id === state.workspaces.current) return;
  state.workspaces.current = id;
  storeWorkspaceId(id);
  state.ui.activeTopic = null;
  state.ui.activeDate = null;
  document.dispatchEvent(new CustomEvent('cta:reload'));
}

/**
 * Regroup the shown workspace after its view mode changed. The selected project carries over:
 * the timeline's project picker becomes the directory's selected project and back.
 */
export function applyViewModeChange() {
  const topic = state.ui.activeTopic;
  state.ui.activeDate = null;
  if (isDirectoryView()) {
    applyListFilters();
    if (topic) selectProject(topic);
    else notifyState();
    return;
  }
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
    const note = await api.resolveDocForOpen(id);
    if (!note.workspace || note.workspace === state.workspaces.current) return false;
    await showWorkspace(note.workspace);
    return true;
  } catch {
    return false;
  }
}
