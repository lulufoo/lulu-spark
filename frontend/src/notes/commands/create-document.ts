import { state } from '../state/host.ts';
import { isDirKey, nodeKeyOf, projectOfNode, sortedProjectFolders } from '../state/dir-tree.ts';
import { createTargetStore } from '../state/create-target.ts';
import { beginDirCreate } from './dir-create.ts';
import { beginTimelineCreate } from './timeline-create.ts';
import { isDirectoryView } from './workspace.ts';

/** The `folder` value the backend reads as "directly in the Project root". */
const PROJECT_ROOT_FOLDER = '/';

/** The enabled projects of the shown workspace, in the order Settings arranged: where a document can go. */
export function createTargets(): string[] {
  return sortedProjectFolders(state.index.topicTitles, state.index.topicOrder);
}

/** The folder the directory view is listing for `project`. The project root when it is not one of its folders. */
function listedFolder(project: string): string {
  const listed = state.ui.activeDate;
  const node = isDirKey(listed) ? nodeKeyOf(listed) : project;
  if (projectOfNode(node) !== project) return PROJECT_ROOT_FOLDER;
  const folders = node.split('/').slice(1);
  return folders.length ? folders.join('/') : PROJECT_ROOT_FOLDER;
}

function defaultProject(targets: string[]): string {
  return targets.find((id) => state.index.topicDefault?.[id] === true) || targets[0] || '';
}

/**
 * The "+" of the document page. Timeline adds a title card in the list: the selected Project,
 * or the workspace default Project when none is selected. Directory view adds a file-name row
 * under the listed folder, and asks which Project when none is selected.
 * Returns false when there is no Project to put a document in.
 */
export function startCreateDocument(): boolean {
  const targets = createTargets();
  if (!targets.length) return false;
  const selected = state.ui.activeTopic;
  const chosen = selected && targets.includes(selected) ? selected : '';
  if (!isDirectoryView()) {
    const project = chosen || defaultProject(targets);
    if (!project) return false;
    beginTimelineCreate(project);
    return true;
  }
  if (chosen) {
    beginDirCreate(chosen, listedFolder(chosen));
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
  if (!project || !createTargets().includes(project)) return;
  if (isDirectoryView()) beginDirCreate(project, PROJECT_ROOT_FOLDER);
  else beginTimelineCreate(project);
}

/** Chooser cancelled: nothing is created. */
export function cancelCreateProject() {
  createTargetStore.set({ open: false, project: '' });
}
