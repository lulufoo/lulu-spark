import { notifyState, state } from '../state/host.ts';
import { loadTitles } from './cards.ts';
import { isDirectoryView } from './workspace.ts';
import { parseHash, navigateToDateList } from '../../router/index.ts';
import { dateKeyOf, UNDATED_KEY } from '../../shared/utils.ts';
import {
  expandDirNodes,
  isDirKey,
  nodeKeyOf,
  nodeKeyOfEntry,
  pathKeys,
  projectOfNode,
  resetDirTree,
  sortedProjectFolders,
} from '../state/dir-tree.ts';
import type { NoteEntry } from '../state/types.ts';

/**
 * Group documents by day, newest day first. Documents without a usable `created_at` are not
 * dropped: they form one `Undated` group after every date, ordered by title.
 */
export function buildGroups(indexData: Record<string, NoteEntry>) {
  const map = new Map<string, { id: string; entry: NoteEntry }[]>();
  for (const [id, entry] of Object.entries(indexData)) {
    if (!entry) continue;
    const date = dateKeyOf(entry.created_at);
    if (!map.has(date)) map.set(date, []);
    map.get(date)!.push({ id, entry });
  }
  const rank = (date: string) => (date === UNDATED_KEY ? 0 : 1);
  return [...map.entries()]
    .sort((a, b) => rank(b[0]) - rank(a[0]) || b[0].localeCompare(a[0]))
    .map(([date, entries]) => ({
      date,
      entries: date === UNDATED_KEY
        ? entries.sort(compareByTitle)
        : entries.sort((a, b) => String(b.entry.created_at).localeCompare(String(a.entry.created_at))),
    }));
}

function compareByTitle(a: { id: string; entry: NoteEntry }, b: { id: string; entry: NoteEntry }) {
  return (a.entry.title || '').localeCompare(b.entry.title || '') || a.id.localeCompare(b.id);
}

/** Timeline view: filter by project. Directory view: the documents directly inside the selected tree node. */
export function applyListFilters() {
  if (isDirectoryView()) {
    state.index.filteredGroups = directoryGroups();
    return;
  }
  let groups = state.index.groupedByDate;
  const topic = state.ui.activeTopic;
  if (topic) {
    groups = groups
      .map(({ date, entries }) => ({
        date,
        entries: entries.filter(({ entry }) => (entry.folder || 'unknown') === topic),
      }))
      .filter(({ entries }) => entries.length > 0);
  }
  state.index.filteredGroups = groups;
}

function directoryGroups() {
  const key = state.ui.activeDate;
  if (!isDirKey(key)) return [];
  const nodeKey = nodeKeyOf(key);
  const entries = Object.entries(state.index.data || {})
    .filter(([, entry]) => entry && nodeKeyOfEntry(entry) === nodeKey)
    .map(([id, entry]) => ({ id, entry }))
    .sort(
      (a, b) =>
        String(b.entry.created_at ?? '').localeCompare(String(a.entry.created_at ?? '')) ||
        String(a.entry.raw_path ?? '').localeCompare(String(b.entry.raw_path ?? '')),
    );
  return [{ date: key, entries }];
}

/** A day key belongs to the timeline view and a `dir:` key to the directory view. */
function fitsCurrentView(date: string): boolean {
  return isDirectoryView() ? isDirKey(date) : !isDirKey(date);
}

export function selectDate(date: string) {
  if (!fitsCurrentView(date)) return;
  const route = parseHash(typeof window !== 'undefined' ? window.location.hash : '');
  if (route.name === 'spark') {
    const params = route.params as { note?: string };
    const hasNote = Boolean(params.note);
    const creating = Boolean(state.viewer?.createSession);
    if (hasNote || creating) {
      if (creating) state.viewer.createSession = null;
      navigateToDateList(date);
      return;
    }
  }

  state.ui.activeDate = date;
  sessionStorage.setItem('cta_active_date', date);
  if (isDirKey(date)) {
    const nodeKey = nodeKeyOf(date);
    expandDirNodes(pathKeys(nodeKey));
    state.ui.activeTopic = projectOfNode(nodeKey);
    applyListFilters();
  }
  notifyState();

  const group = state.index.filteredGroups.find((g) => g.date === date);
  if (!group) return;
  loadTitles(group.entries, date);
}

/** Select a project (Home sidebar or the picker): the tree shows its content, or the timeline filters to it. */
export function selectProject(folder: string) {
  if (!isDirectoryView()) {
    selectTopic(folder);
    return;
  }
  state.ui.activeTopic = folder;
  state.ui.activeDate = null;
  resetDirTree();
  applyListFilters();
  notifyState();
}

/**
 * Entering a workspace without a chosen project. The directory view always has a project in its
 * dropdown, so it takes the first one; the timeline view shows every project (All).
 */
export function selectDefaultProject() {
  if (!isDirectoryView()) {
    selectTopic(null);
    return;
  }
  const [first] = sortedProjectFolders(state.index.topicTitles, state.index.topicOrder);
  if (first) {
    selectProject(first);
    return;
  }
  state.ui.activeTopic = null;
  state.ui.activeDate = null;
  resetDirTree();
  applyListFilters();
  notifyState();
}

export function selectTopic(key: string | null) {
  state.ui.activeTopic = key;
  applyListFilters();
  if (state.index.filteredGroups.length > 0) {
    selectDate(state.index.filteredGroups[0].date);
  } else {
    state.ui.activeDate = null;
    notifyState();
  }
}
