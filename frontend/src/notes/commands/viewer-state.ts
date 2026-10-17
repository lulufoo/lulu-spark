import * as api from '../../host/api.ts';
import { state } from '../state/host.ts';
import { nodeKeyOfEntry } from '../state/dir-tree.ts';
import type { NoteEntry } from '../state/types.ts';
import { isDirectoryView } from './workspace.ts';

/**
 * What each workspace's page had selected, per Project. The backend keeps it in a file; this
 * module holds a copy per workspace (read when the workspace's index loads) so entering a
 * workspace or Project can restore synchronously, and writes to the file in call order.
 */
type Memory = api.ViewerState;
type Patch = { project: string; node?: string; doc?: string; date?: string };

const cache = new Map<string, Memory>();
let quiet = 0;
let writes: Promise<unknown> = Promise.resolve();

const empty = (): Memory => ({ version: 2, project: '', projects: {} });

/** What `workspace` remembers; nothing remembered reads as empty. */
export function remembered(workspace: string = state.workspaces.current): Memory {
  return cache.get(workspace) ?? empty();
}

function parse(raw: unknown): Memory {
  const rec = raw as Partial<Memory> | null;
  const out = empty();
  if (!rec || typeof rec !== 'object') return out;
  out.project = typeof rec.project === 'string' ? rec.project : '';
  for (const [folder, value] of Object.entries(rec.projects ?? {})) {
    const part = (key: 'node' | 'doc' | 'date') => (typeof value?.[key] === 'string' ? value[key] : '');
    out.projects[folder] = { node: part('node'), doc: part('doc'), date: part('date') };
  }
  return out;
}

/** Read `workspace`'s memory from the backend; a failed read is the same as nothing remembered. */
export async function loadViewerMemory(workspace: string): Promise<void> {
  await writes;
  try {
    cache.set(workspace, parse(await api.fetchViewerState(workspace)));
  } catch {
    cache.set(workspace, empty());
  }
}

/** Resolves once every write made so far has reached the backend. */
export async function flushViewerWrites(): Promise<void> {
  await writes;
}

/** Run a restore: the selections it makes are not new choices and must not overwrite the record. */
export function whileRestoring<T>(fn: () => T): T {
  quiet += 1;
  try {
    return fn();
  } finally {
    quiet -= 1;
  }
}

/** Mirror of the backend's patch rules: left out = kept, `''` = cleared, All has no entry. */
function applyPatch(memory: Memory, patch: Patch) {
  memory.project = patch.project;
  if (!patch.project) return;
  const entry = { ...(memory.projects[patch.project] ?? { node: '', doc: '', date: '' }) };
  for (const key of ['node', 'doc', 'date'] as const) {
    if (patch[key] !== undefined) entry[key] = patch[key];
  }
  if (entry.node || entry.doc || entry.date) memory.projects[patch.project] = entry;
  else delete memory.projects[patch.project];
}

/** Best-effort: a failed cache write must never get in the way of reading. */
function record(patch: Patch) {
  if (quiet) return;
  const workspace = state.workspaces.current;
  const memory = cache.get(workspace) ?? empty();
  cache.set(workspace, memory);
  applyPatch(memory, patch);
  writes = writes.then(() => api.saveViewerState(workspace, patch)).catch(() => {});
}

/** A project (or All, `''`) became the selected one. */
export function rememberProject(project: string) {
  record({ project });
}

/** Directory view: a tree node was selected. A document remembered under another node is dropped. */
export function rememberNode(project: string, node: string) {
  const known = remembered().projects[project];
  record(known && known.node !== node ? { project, node, doc: '' } : { project, node });
}

/** Timeline: a day was selected (All has no entry, only the project is remembered). */
export function rememberDate(project: string, date: string) {
  record({ project, date });
}

/** Directory view: the document just opened is the one its project reopens next time. */
export function rememberOpenDoc(entry: NoteEntry, id: string) {
  const project = entry.project_id || '';
  if (!id || !project || !isDirectoryView()) return;
  record({ project, node: nodeKeyOfEntry(entry), doc: id });
}

/** Closing the viewer forgets the document: its project restores only the folder. */
export function forgetOpenDoc() {
  const project = state.ui.activeTopic;
  if (!project || !isDirectoryView()) return;
  record({ project, doc: '' });
}
