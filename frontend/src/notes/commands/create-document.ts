import { state } from '../state/host.ts';
import { sortedProjectFolders } from '../state/dir-tree.ts';
import { createTargetStore } from '../state/create-target.ts';
import { openCreateNote } from './viewer/create.ts';

/** The enabled projects of the shown workspace, in the order Settings arranged: where a document can go. */
export function createTargets(): string[] {
  return sortedProjectFolders(state.index.topicTitles, state.index.topicOrder);
}

function newTempId(): string {
  return typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function'
    ? crypto.randomUUID()
    : `note-${Date.now()}`;
}

function beginCreate(project: string) {
  void openCreateNote({ temp_id: newTempId(), workspace: state.workspaces.current, project });
}

/**
 * The "+" of the document page. A document always goes to an explicit workspace and Project: the
 * selected Project when there is one, otherwise (timeline All) the person picks one first.
 * Returns false when there is no Project to put a document in.
 */
export function startCreateDocument(): boolean {
  const targets = createTargets();
  if (!targets.length) return false;
  const selected = state.ui.activeTopic;
  if (selected && targets.includes(selected)) {
    beginCreate(selected);
    return true;
  }
  createTargetStore.set({ open: true, project: targets[0] });
  return true;
}

export function chooseCreateProject(project: string) {
  createTargetStore.set((prev) => ({ ...prev, project }));
}

/** Chooser confirmed: start the document in the picked Project. */
export function confirmCreateProject() {
  const { project } = createTargetStore.getSnapshot();
  createTargetStore.set({ open: false, project: '' });
  if (project && createTargets().includes(project)) beginCreate(project);
}

/** Chooser cancelled: nothing is created. */
export function cancelCreateProject() {
  createTargetStore.set({ open: false, project: '' });
}
