import type * as api from '../../host/api.ts';
import { parseHash } from '../../router/index.ts';
import { state } from '../state/host.ts';
import { buildDirTree, dirKeyOf, findDirNode, projectOfNode, sortedProjectFolders } from '../state/dir-tree.ts';
import { selectDate, selectDefaultProject, selectProject, selectTopic } from './sidebar.ts';
import { rememberProject, remembered, whileRestoring } from './viewer-state.ts';
import { isDirectoryView } from './workspace.ts';

/**
 * Entering a workspace or a Project puts back what that Project had selected. A remembered thing
 * that is gone (document deleted or hidden, folder removed, day without data) degrades silently:
 * document → its folder, folder → no record, day → the latest day.
 */

function onListRoute(): boolean {
  const route = parseHash(typeof window !== 'undefined' ? window.location.hash : '');
  return route.name === 'spark' && !(route.params as { note?: string }).note;
}

function openRememberedDoc(project: string, id: string) {
  if (!id || !onListRoute()) return;
  const data = state.index.data || {};
  const known = data[id] ?? Object.values(data).find((e) => e._id === id || e.id === id);
  if (!known || (known.folder || '') !== project) return;
  document.dispatchEvent(new CustomEvent('cta:open-entry', { detail: { id } }));
}

function enterDirectoryProject(folder: string, entry: api.ViewerProjectState | undefined) {
  selectProject(folder);
  const node = entry?.node ?? '';
  if (!node || projectOfNode(node) !== folder) return '';
  if (!findDirNode(buildDirTree(state.index.data, state.index.topicTitles), node)) return '';
  selectDate(dirKeyOf(node));
  return entry?.doc ?? '';
}

function enterTimelineProject(folder: string | null, entry: api.ViewerProjectState | undefined) {
  selectTopic(folder);
  const date = entry?.date ?? '';
  if (date && state.index.filteredGroups.some((g) => g.date === date)) selectDate(date);
}

/** Enter a project of the shown workspace (`null` = All in the timeline) and restore its record. */
export function enterProject(folder: string | null) {
  const entry = folder ? remembered().projects[folder] : undefined;
  let doc = '';
  whileRestoring(() => {
    if (isDirectoryView()) doc = folder ? enterDirectoryProject(folder, entry) : '';
    else enterTimelineProject(folder, entry);
  });
  rememberProject(folder ?? '');
  if (folder && doc) openRememberedDoc(folder, doc);
}

/**
 * Enter the shown workspace: its last selected Project, or the first one in the order Settings
 * arranged (the directory view always has a project, the timeline view starts at All).
 */
export function enterWorkspace() {
  const titles = state.index.topicTitles;
  const last = remembered().project;
  const first = sortedProjectFolders(titles, state.index.topicOrder)[0] ?? null;
  if (isDirectoryView()) {
    const project = last && last in titles ? last : first;
    if (project) enterProject(project);
    else selectDefaultProject();
    return;
  }
  enterProject(last ? (last in titles ? last : first) : null);
}
