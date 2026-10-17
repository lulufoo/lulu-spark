import * as api from '../../../host/api.ts';
import { withDisabledInPlace } from '../../../shared/reorder.ts';
import { announceRegistryChange } from '../../../shared/registry-events.ts';
import { notifyState, state } from '../../../host/state.ts';
import {
  emptyNotesCategoriesSnap,
  notesCategoriesStore,
  patchNotesCategories,
  type NotesCategoryRow,
} from '../../state/settings/notes-categories.ts';

/** One row per project; a project is addressed by its folder. */
function asRows(data: unknown): NotesCategoryRow[] {
  const rec = data as { projects?: Array<Partial<NotesCategoryRow> & { sort_index?: number }>; error?: string };
  if (rec?.error) throw new Error(rec.error);
  const rows: NotesCategoryRow[] = [];
  for (const [at, c] of (rec.projects || []).entries()) {
    const folder = typeof c.folder === 'string' ? c.folder.trim() : '';
    if (!folder) continue;
    rows.push({
      id: folder,
      title: typeof c.title === 'string' ? c.title : folder,
      description: typeof c.description === 'string' ? c.description : '',
      folder,
      icon: typeof c.icon === 'string' ? c.icon : '',
      sortIndex: typeof c.sort_index === 'number' ? c.sort_index : at,
    });
  }
  return rows;
}

function throwOnError(data: unknown) {
  const error = (data as { error?: string } | null)?.error;
  if (error) throw new Error(error);
}

function failWith(e: unknown) {
  patchNotesCategories({ error: (e as Error).message || String(e), busy: false });
}

/** The page shows project titles of the workspace it browses; keep them in step with edits. */
function applyNotesCategoryMaps(rows: NotesCategoryRow[]) {
  const descMap: Record<string, string> = {};
  const titleMap: Record<string, string> = {};
  const order: string[] = [];
  for (const row of rows) {
    const key = row.folder || row.id;
    order.push(key);
    titleMap[key] = row.title || key;
    descMap[key] = row.description || '';
  }
  state.index.topicTitles = titleMap;
  state.index.topicOrder = order;
  state.index.topicDescriptions = descMap;
  state.index.topicRepos = {};
  notifyState();
}

function rowById(rows: NotesCategoryRow[], id: string) {
  return rows.find((row) => row.id === id);
}

/** Reload both lists of `workspaceId`; a late answer for a workspace that is no longer selected is dropped. */
export async function loadNotesCategories(workspaceId: string) {
  const snap = notesCategoriesStore.getSnapshot();
  const keepId = snap.editorOpen && snap.workspaceId === workspaceId ? snap.selectedId : '';
  patchNotesCategories({ error: '', busy: true });
  try {
    const rows = asRows(await api.fetchProjects(workspaceId));
    const disabled = asRows(await api.fetchDisabledProjects(workspaceId));
    if (notesCategoriesStore.getSnapshot().workspaceId !== workspaceId) return;
    if (workspaceId === state.workspaces.current) applyNotesCategoryMaps(rows);
    const selected = keepId ? rowById(rows, keepId) : undefined;
    patchNotesCategories({
      rows,
      disabled,
      selectedId: selected?.id || '',
      draftTitle: selected?.title || '',
      draftDescription: selected?.description || '',
      draftIcon: selected?.icon || '',
      busy: false,
      editorOpen: !!selected,
      confirmDisableId: '',
    });
  } catch (e) {
    failWith(e);
  }
}

/** Show the projects of another workspace (a settings nav item was chosen). */
export async function selectProjectsWorkspace(workspaceId: string) {
  patchNotesCategories({
    ...emptyNotesCategoriesSnap(),
    workspaceId,
    busy: true,
  });
  await loadNotesCategories(workspaceId);
}

/**
 * Drag-reorder: show the new order at once, then ask the backend; a refusal puts the old order
 * back and says why. `folders` are the enabled projects in their new order, disabled ones keep
 * their place.
 */
export async function reorderNotesCategories(workspaceId: string, folders: string[]): Promise<boolean> {
  const snap = notesCategoriesStore.getSnapshot();
  const byId = new Map(snap.rows.map((row) => [row.id, row]));
  const next = folders.map((id) => byId.get(id)).filter((row): row is NotesCategoryRow => Boolean(row));
  if (snap.workspaceId !== workspaceId || next.length !== snap.rows.length) return false;
  const whole = withDisabledInPlace(snap.rows.map((r) => ({ id: r.id, sortIndex: r.sortIndex })), snap.disabled.map((r) => ({ id: r.id, sortIndex: r.sortIndex })), folders);
  patchNotesCategories({ rows: next, error: '' });
  try {
    throwOnError(await api.reorderProjects(workspaceId, whole));
    await loadNotesCategories(workspaceId);
    announceRegistryChange();
    return true;
  } catch (e) {
    if (notesCategoriesStore.getSnapshot().workspaceId === workspaceId) {
      patchNotesCategories({ rows: snap.rows, error: `Could not change the order: ${(e as Error).message || String(e)}` });
    }
    return false;
  }
}

export function openNotesCategoryEditor(id: string) {
  const selected = rowById(notesCategoriesStore.getSnapshot().rows, id);
  if (!selected) return;
  patchNotesCategories({
    selectedId: selected.id,
    draftTitle: selected.title,
    draftDescription: selected.description,
    draftIcon: selected.icon,
    error: '',
    editorOpen: true,
    confirmDisableId: '',
  });
}

export function closeNotesCategoryEditor() {
  const snap = notesCategoriesStore.getSnapshot();
  if (!snap.editorOpen && !snap.selectedId && !snap.confirmDisableId) return;
  patchNotesCategories({
    selectedId: '',
    draftTitle: '',
    draftDescription: '',
    draftIcon: '',
    error: '',
    editorOpen: false,
    confirmDisableId: '',
  });
}

/** `inbox` is the built-in project and cannot be disabled. */
export function askNotesCategoryDisable(id: string) {
  if (!id || id === 'inbox') return;
  patchNotesCategories({ confirmDisableId: id, error: '' });
}

export function cancelNotesCategoryDisable() {
  patchNotesCategories({ confirmDisableId: '' });
}

export function setNotesCategoryDraft(title: string, description: string) {
  patchNotesCategories({ draftTitle: title, draftDescription: description });
}

export function setNotesCategoryDraftIcon(icon: string) {
  patchNotesCategories({ draftIcon: icon });
}

/** `icon` empty = the default project icon. */
export async function addNotesCategory(workspaceId: string, title: string, description: string, icon = '') {
  patchNotesCategories({ error: '', busy: true });
  try {
    throwOnError(
      await (icon
        ? api.createProject(workspaceId, title, description, icon)
        : api.createProject(workspaceId, title, description)),
    );
    await loadNotesCategories(workspaceId);
    announceRegistryChange();
  } catch (e) {
    failWith(e);
  }
}

export async function saveNotesCategory(workspaceId: string) {
  const snap = notesCategoriesStore.getSnapshot();
  if (!snap.selectedId || !snap.draftTitle.trim()) return;
  const current = rowById(snap.rows, snap.selectedId);
  const iconChanged = snap.draftIcon !== (current?.icon ?? '');
  patchNotesCategories({ error: '', busy: true });
  try {
    throwOnError(
      await (iconChanged
        ? api.updateProject(workspaceId, snap.selectedId, snap.draftTitle, snap.draftDescription, snap.draftIcon)
        : api.updateProject(workspaceId, snap.selectedId, snap.draftTitle, snap.draftDescription)),
    );
    closeNotesCategoryEditor();
    await loadNotesCategories(workspaceId);
    announceRegistryChange();
  } catch (e) {
    failWith(e);
  }
}

export async function disableNotesCategory(workspaceId: string, id: string) {
  patchNotesCategories({ error: '', busy: true });
  try {
    throwOnError(await api.disableProject(workspaceId, id));
    closeNotesCategoryEditor();
    await loadNotesCategories(workspaceId);
    announceRegistryChange();
  } catch (e) {
    failWith(e);
  }
}

export async function enableNotesCategory(workspaceId: string, id: string) {
  patchNotesCategories({ error: '', busy: true });
  try {
    throwOnError(await api.enableProject(workspaceId, id));
    await loadNotesCategories(workspaceId);
    announceRegistryChange();
  } catch (e) {
    failWith(e);
  }
}
