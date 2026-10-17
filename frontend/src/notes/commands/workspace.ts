import * as api from '../../host/api.ts';
import { notifyState, state } from '../state/host.ts';
import type { HostWorkspace } from '../../host/state.ts';
import { dateKeyOf } from '../../shared/utils.ts';
import { dirKeyOf, nodeKeyOfEntry } from '../state/dir-tree.ts';
import type { NoteEntry } from '../state/types.ts';

const CURRENT_WORKSPACE_KEY = 'spark_workspace';
const DEFAULT_WORKSPACE_ID = 'default';

function parseWorkspace(raw: unknown): HostWorkspace | null {
  const rec = raw as Partial<HostWorkspace> | null;
  if (!rec || typeof rec.id !== 'string' || !rec.id) return null;
  return {
    id: rec.id,
    name: typeof rec.name === 'string' && rec.name ? rec.name : rec.id,
    view_mode: rec.view_mode === 'timeline' ? 'timeline' : 'directory',
    is_default: Boolean(rec.is_default),
    icon: typeof rec.icon === 'string' ? rec.icon : '',
  };
}

export function getCurrentWorkspace(): HostWorkspace | undefined {
  const { current, list } = state.workspaces;
  return list.find((w) => w.id === current);
}

/** Timeline view: documents grouped by day, with a project picker. */
export function isTimelineView(): boolean {
  return !isDirectoryView();
}

/**
 * Directory view: a tree of project → sub-folder → document. Until the workspace registry has
 * loaded the page falls back to the timeline grouping.
 */
export function isDirectoryView(): boolean {
  return getCurrentWorkspace()?.view_mode === 'directory';
}

/** Sidebar group a document belongs to: its day (timeline) or its tree node (directory). */
export function entryGroupKey(entry: NoteEntry): string {
  return isDirectoryView() ? dirKeyOf(nodeKeyOfEntry(entry)) : dateKeyOf(entry.created_at);
}

export function readStoredWorkspaceId(): string {
  try {
    return localStorage.getItem(CURRENT_WORKSPACE_KEY) || DEFAULT_WORKSPACE_ID;
  } catch {
    return DEFAULT_WORKSPACE_ID;
  }
}

export function storeWorkspaceId(id: string) {
  try {
    localStorage.setItem(CURRENT_WORKSPACE_KEY, id);
  } catch {
    // Not fatal: the next launch falls back to the default workspace.
  }
}

/** Load the workspace registry; the page keeps the remembered workspace when it still exists. */
export async function loadWorkspaces(): Promise<void> {
  const data = (await api.fetchWorkspaces()) as { workspaces?: unknown[]; error?: string };
  if (data?.error) throw new Error(data.error);
  const list = (data?.workspaces || []).map(parseWorkspace).filter((w): w is HostWorkspace => w !== null);
  state.workspaces.list = list;
  const wanted = state.workspaces.current !== DEFAULT_WORKSPACE_ID
    ? state.workspaces.current
    : readStoredWorkspaceId();
  const fallback = list.find((w) => w.is_default)?.id || DEFAULT_WORKSPACE_ID;
  state.workspaces.current = list.some((w) => w.id === wanted) ? wanted : fallback;
  notifyState();
}

/** Project titles of the current workspace, keyed by project folder. */
export async function loadProjects(): Promise<void> {
  const data = (await api.fetchProjects(state.workspaces.current)) as {
    projects?: Array<{ folder?: string; title?: string; description?: string }>;
    error?: string;
  };
  if (data?.error) throw new Error(data.error);
  const titles: Record<string, string> = {};
  const descriptions: Record<string, string> = {};
  for (const project of data?.projects || []) {
    const folder = typeof project.folder === 'string' ? project.folder : '';
    if (!folder) continue;
    titles[folder] = typeof project.title === 'string' && project.title ? project.title : folder;
    descriptions[folder] = typeof project.description === 'string' ? project.description : '';
  }
  state.index.topicTitles = titles;
  state.index.topicDescriptions = descriptions;
  state.index.topicRepos = {};
  notifyState();
}
