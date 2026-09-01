import { createModuleStore } from '../../../shared/module-store.ts';

export type NotesCategoryRow = {
  id: string;
  title: string;
  description: string;
  folder: string;
};

export type NotesCategoriesSnap = {
  rows: NotesCategoryRow[];
  selectedId: string;
  draftTitle: string;
  draftDescription: string;
  error: string;
  busy: boolean;
  editorOpen: boolean;
  confirmDelete: boolean;
};

export function emptyNotesCategoriesSnap(): NotesCategoriesSnap {
  return {
    rows: [],
    selectedId: '',
    draftTitle: '',
    draftDescription: '',
    error: '',
    busy: false,
    editorOpen: false,
    confirmDelete: false,
  };
}

export const notesCategoriesStore = createModuleStore<NotesCategoriesSnap>(emptyNotesCategoriesSnap());

export function patchNotesCategories(partial: Partial<NotesCategoriesSnap>) {
  notesCategoriesStore.set({ ...notesCategoriesStore.getSnapshot(), ...partial });
}
