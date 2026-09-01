import * as api from '../../../host/api.ts';
import { notifyState, state } from '../../../host/state.ts';
import {
  notesCategoriesStore,
  patchNotesCategories,
  type NotesCategoryRow,
} from '../../state/settings/notes-categories.ts';

function asRows(data: unknown): NotesCategoryRow[] {
  const rec = data as { categories?: Array<Partial<NotesCategoryRow>> };
  const rows: NotesCategoryRow[] = [];
  for (const c of rec.categories || []) {
    const id = typeof c.id === 'string' ? c.id.trim() : '';
    if (!id) continue;
    const folder =
      typeof c.folder === 'string' && c.folder.trim() ? c.folder.trim() : id;
    rows.push({
      id,
      title: typeof c.title === 'string' ? c.title : id,
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
    const rows = asRows(await api.fetchNotesCategories());
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
    const data = (await api.createNotesCategory(title, description)) as {
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
    const data = (await api.updateNotesCategory(
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
    const data = (await api.deleteNotesCategory(id)) as { error?: string };
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
