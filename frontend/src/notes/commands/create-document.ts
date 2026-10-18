import { state } from '../state/host.ts';
import { isDirKey, nodeKeyOf, projectOfNode, sortedProjectFolders } from '../state/dir-tree.ts';
import { createTargetStore } from '../state/create-target.ts';
import { openCreateNote } from './viewer/create.ts';
import { isDirectoryView } from './workspace.ts';

/** The `folder` value the backend reads as "directly in the Project root". */
const PROJECT_ROOT_FOLDER = '/';

/** The enabled projects of the shown workspace, in the order Settings arranged: where a document can go. */
export function createTargets(): string[] {
  return sortedProjectFolders(state.index.topicTitles, state.index.topicOrder);
}

function newTempId(): string {
  return typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function'
    ? crypto.randomUUID()
    : `note-${Date.now()}`;
}

/**
 * The folder of `project` a document created now goes into. The directory view lists one folder at
 * a time, so that is the one (the Project root when the root is listed or nothing else is).
 * The timeline view has no folder: nothing is sent and the backend picks its default.
 */
function createFolder(project: string): string | undefined {
  if (!isDirectoryView()) return undefined;
  const listed = state.ui.activeDate;
  const node = isDirKey(listed) ? nodeKeyOf(listed) : project;
  if (projectOfNode(node) !== project) return PROJECT_ROOT_FOLDER;
  const folders = node.split('/').slice(1);
  return folders.length ? folders.join('/') : PROJECT_ROOT_FOLDER;
}

function beginCreate(project: string) {
  void openCreateNote({
    temp_id: newTempId(),
    workspace_id: state.workspaces.current,
    project_id: project,
    folder: createFolder(project),
  });
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
