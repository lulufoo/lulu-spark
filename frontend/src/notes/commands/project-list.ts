import * as api from '../../host/api.ts';
import { notifyState, state } from '../state/host.ts';
import { normalizeDocIndex } from '../state/doc-index.ts';
import type { NoteEntry } from '../state/types.ts';
import { applyListFilters, buildGroups } from './sidebar.ts';

/** One line per document in `projectId`: id and path, sorted, so two lists can be compared. */
export function projectListLines(data: Record<string, NoteEntry> | null | undefined, projectId: string): string[] {
  const lines: string[] = [];
  for (const [key, entry] of Object.entries(data || {})) {
    if (!entry || (entry.project_id || '') !== projectId) continue;
    lines.push(`${entry.id || entry._id || key}\t${entry.raw_path || ''}`);
  }
  lines.sort();
  return lines;
}

export function sameProjectList(memory: string[], local: string[]): boolean {
  if (memory.length !== local.length) return false;
  for (let i = 0; i < memory.length; i += 1) {
    if (memory[i] !== local[i]) return false;
  }
  return true;
}

/**
 * Opening a project: read the local index and compare this project's list with memory.
 * Same list: leave the screen alone. A different list replaces the in-memory index.
 */
export async function reloadProjectListIfStale(projectId: string): Promise<void> {
  const workspaceId = state.workspaces.current;
  let fresh: Record<string, NoteEntry>;
  try {
    fresh = normalizeDocIndex(await api.fetchIndex(workspaceId)) as Record<string, NoteEntry>;
  } catch {
    return;
  }
  if (state.workspaces.current !== workspaceId || state.ui.activeTopic !== projectId) return;
  if (sameProjectList(projectListLines(state.index.data, projectId), projectListLines(fresh, projectId))) return;
  state.index.data = fresh;
  state.index.groupedByDate = buildGroups(fresh);
  applyListFilters();
  notifyState();
}
