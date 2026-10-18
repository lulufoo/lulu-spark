import * as api from '../../../host/api.ts';
import { announceRegistryChange } from '../../../shared/registry-events.ts';
import { notifyState, state } from '../../../host/state.ts';
import {
  emptyNotesCategoriesSnap,
  notesCategoriesStore,
  patchNotesCategories,
  type NotesCategoryRow,
} from '../../state/settings/notes-categories.ts';

/** One row per project; a project is addressed by its id. */
function asRows(data: unknown): NotesCategoryRow[] {
  const rec = data as {
    projects?: Array<{
      project_id?: unknown;
      project_name?: unknown;
      description?: unknown;
      icon?: unknown;
      is_inbox?: unknown;
      doc_count?: unknown;
    }>;
    error?: string;
  };
  if (rec?.error) throw new Error(rec.error);
  const rows: NotesCategoryRow[] = [];
  for (const c of rec.projects || []) {
    const id = typeof c.project_id === 'string' ? c.project_id.trim() : '';
    if (!id) continue;
    rows.push({
      id,
      title: typeof c.project_name === 'string' && c.project_name ? c.project_name : id,
      description: typeof c.description === 'string' ? c.description : '',
      is_inbox: c.is_inbox === true,
      icon: typeof c.icon === 'string' ? c.icon : '',
      doc_count: typeof c.doc_count === 'number' ? c.doc_count : 0,
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
  const inboxMap: Record<string, boolean> = {};
  const order: string[] = [];
  for (const row of rows) {
    const key = row.id;
    order.push(key);
    titleMap[key] = row.title || key;
    descMap[key] = row.description || '';
    inboxMap[key] = row.is_inbox;
  }
  state.index.topicTitles = titleMap;
  state.index.topicOrder = order;
  state.index.topicDescriptions = descMap;
  state.index.topicInbox = inboxMap;
  state.index.topicRepos = {};
  notifyState();
}

function rowById(rows: NotesCategoryRow[], id: string) {
  return rows.find((row) => row.id === id);
}

/** Reload the projects of `workspaceId`; a late answer for a workspace that is no longer selected is dropped. */
export async function loadNotesCategories(workspaceId: string) {
  const snap = notesCategoriesStore.getSnapshot();
  const keepId = snap.editorOpen && snap.workspaceId === workspaceId ? snap.selectedId : '';
  patchNotesCategories({ error: '', busy: true });
  try {
    const rows = asRows(await api.fetchProjects(workspaceId));
    if (notesCategoriesStore.getSnapshot().workspaceId !== workspaceId) return;
    if (workspaceId === state.workspaces.current) applyNotesCategoryMaps(rows);
    const selected = keepId ? rowById(rows, keepId) : undefined;
    patchNotesCategories({
      rows,
      selectedId: selected?.id || '',
      draftTitle: selected?.title || '',
      draftDescription: selected?.description || '',
      draftIcon: selected?.icon || '',
      busy: false,
      editorOpen: !!selected,
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
 * back and says why. `ids` are every project of the workspace in their new order.
 */
export async function reorderNotesCategories(workspaceId: string, ids: string[]): Promise<boolean> {
  const snap = notesCategoriesStore.getSnapshot();
  const byId = new Map(snap.rows.map((row) => [row.id, row]));
  const next = ids.map((id) => byId.get(id)).filter((row): row is NotesCategoryRow => Boolean(row));
  if (snap.workspaceId !== workspaceId || next.length !== snap.rows.length) return false;
  patchNotesCategories({ rows: next, error: '' });
  try {
    throwOnError(await api.reorderProjects(workspaceId, ids));
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
  });
}

export function closeNotesCategoryEditor() {
  const snap = notesCategoriesStore.getSnapshot();
  if (!snap.editorOpen && !snap.selectedId) return;
  patchNotesCategories({
    selectedId: '',
    draftTitle: '',
    draftDescription: '',
    draftIcon: '',
    error: '',
    editorOpen: false,
  });
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
