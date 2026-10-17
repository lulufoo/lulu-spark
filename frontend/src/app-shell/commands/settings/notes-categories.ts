import * as api from '../../../host/api.ts';
import { notifyState, state } from '../../../host/state.ts';
import {
  notesCategoriesStore,
  patchNotesCategories,
  type NotesCategoryRow,
} from '../../state/settings/notes-categories.ts';

/** One row per project of the current workspace; a project is addressed by its folder. */
function asRows(data: unknown): NotesCategoryRow[] {
  const rec = data as { projects?: Array<Partial<NotesCategoryRow>>; error?: string };
  if (rec?.error) throw new Error(rec.error);
  const rows: NotesCategoryRow[] = [];
  for (const c of rec.projects || []) {
    const folder = typeof c.folder === 'string' ? c.folder.trim() : '';
    if (!folder) continue;
    rows.push({
      id: folder,
      title: typeof c.title === 'string' ? c.title : folder,
      description: typeof c.description === 'string' ? c.description : '',
      folder,
    });
  }
  return rows;
}

export function applyNotesCategoryMaps(rows: NotesCategoryRow[]) {
  const descMap: Record<string, string> = {};
  const titleMap: Record<string, string> = {};
  for (const row of rows) {
    const key = row.folder || row.id;
    titleMap[key] = row.title || key;
    descMap[key] = row.description || '';
  }
  state.index.topicTitles = titleMap;
  state.index.topicDescriptions = descMap;
  state.index.topicRepos = {};
  notifyState();
}

function rowById(rows: NotesCategoryRow[], id: string) {
  return rows.find((row) => row.id === id);
}

export async function loadNotesCategories() {
  const snap = notesCategoriesStore.getSnapshot();
  const keepId = snap.editorOpen ? snap.selectedId : '';
  patchNotesCategories({ error: '', busy: true });
  try {
    const rows = asRows(await api.fetchProjects(state.workspaces.current));
    applyNotesCategoryMaps(rows);
    const selected = keepId ? rowById(rows, keepId) : undefined;
    patchNotesCategories({
      rows,
      selectedId: selected?.id || '',
      draftTitle: selected?.title || '',
      draftDescription: selected?.description || '',
      busy: false,
      editorOpen: !!selected,
      confirmDelete: false,
    });
  } catch (e) {
    patchNotesCategories({
      error: (e as Error).message || String(e),
      busy: false,
    });
  }
}

export function openNotesCategoryEditor(id: string) {
  const selected = rowById(notesCategoriesStore.getSnapshot().rows, id);
  if (!selected) return;
  patchNotesCategories({
    selectedId: selected.id,
    draftTitle: selected.title,
    draftDescription: selected.description,
    error: '',
    editorOpen: true,
    confirmDelete: false,
  });
}

export function closeNotesCategoryEditor() {
  const snap = notesCategoriesStore.getSnapshot();
  if (!snap.editorOpen && !snap.selectedId && !snap.confirmDelete) return;
  patchNotesCategories({
    selectedId: '',
    draftTitle: '',
    draftDescription: '',
    error: '',
    editorOpen: false,
    confirmDelete: false,
  });
}

export function askNotesCategoryDelete() {
  const snap = notesCategoriesStore.getSnapshot();
  if (!snap.selectedId || snap.selectedId === 'inbox') return;
  patchNotesCategories({ confirmDelete: true, error: '' });
}

export function cancelNotesCategoryDelete() {
  patchNotesCategories({ confirmDelete: false });
}

export function clearNotesCategoryError() {
  if (!notesCategoriesStore.getSnapshot().error) return;
  patchNotesCategories({ error: '' });
}

export function setNotesCategoryDraft(title: string, description: string) {
  patchNotesCategories({ draftTitle: title, draftDescription: description });
}

export async function addNotesCategory(title: string, description: string) {
  patchNotesCategories({ error: '', busy: true });
  try {
    const data = (await api.createProject(state.workspaces.current, title, description)) as {
      error?: string;
    };
    if (data.error) throw new Error(data.error);
    await loadNotesCategories();
  } catch (e) {
    patchNotesCategories({
      error: (e as Error).message || String(e),
      busy: false,
    });
  }
}

export async function saveNotesCategory() {
  const snap = notesCategoriesStore.getSnapshot();
  if (!snap.selectedId || !snap.draftTitle.trim()) return;
  patchNotesCategories({ error: '', busy: true });
  try {
    const data = (await api.updateProject(
      state.workspaces.current,
      snap.selectedId,
      snap.draftTitle,
      snap.draftDescription,
    )) as { error?: string };
    if (data.error) throw new Error(data.error);
    closeNotesCategoryEditor();
    await loadNotesCategories();
  } catch (e) {
    patchNotesCategories({
      error: (e as Error).message || String(e),
      busy: false,
    });
  }
}

export async function removeNotesCategory(id: string) {
  patchNotesCategories({ error: '', busy: true });
  try {
    const data = (await api.disableProject(state.workspaces.current, id)) as { error?: string };
    if (data.error) throw new Error(data.error);
    closeNotesCategoryEditor();
    await loadNotesCategories();
  } catch (e) {
    patchNotesCategories({
      error: (e as Error).message || String(e),
      busy: false,
    });
  }
}
