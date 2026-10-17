import * as api from '../../host/api.ts';
import { parseHash } from '../../router/index.ts';
import { state } from '../state/host.ts';
import { isDirectoryView } from './workspace.ts';

/** Best-effort: a failed cache write must never get in the way of reading. */
async function save(workspace: string, id: string) {
  try {
    await api.saveViewerState(workspace, { project: state.ui.activeTopic ?? '', doc: id });
  } catch {
    /* ignore */
  }
}

async function load(workspace: string): Promise<string> {
  try {
    const rec = (await api.fetchViewerState(workspace)) as Partial<api.ViewerState> | null;
    const doc = rec?.projects?.[rec.project ?? '']?.doc;
    return typeof doc === 'string' ? doc.trim() : '';
  } catch {
    return '';
  }
}

/** The document just opened is the one this workspace's page reopens next time. */
export function rememberOpenDoc(id: string) {
  if (id) void save(state.workspaces.current, id);
}

/** Closing the viewer forgets it: a page without an open document restores nothing. */
export function forgetOpenDoc() {
  void save(state.workspaces.current, '');
}

function onListRoute(): boolean {
  const route = parseHash();
  return route.name === 'spark' && !(route.params as { note?: string }).note;
}

/**
 * Entering a directory-view workspace on its list page reopens the document it had open.
 * A document that no longer exists (deleted, or now hidden) is forgotten instead.
 * Other views and routes that already name a document are left alone.
 */
export async function restoreOpenDoc() {
  if (!isDirectoryView() || !onListRoute()) return;
  const workspace = state.workspaces.current;
  const id = await load(workspace);
  if (!id) return;
  // The user may have moved on while the cache was read.
  if (workspace !== state.workspaces.current || !isDirectoryView() || !onListRoute()) return;
  const data = state.index.data || {};
  const known = data[id] ?? Object.values(data).find((e) => e._id === id || e.id === id);
  if (!known) {
    await save(workspace, '');
    return;
  }
  document.dispatchEvent(new CustomEvent('cta:open-entry', { detail: { id } }));
}
