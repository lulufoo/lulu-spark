import { createModuleStore } from '../../../shared/module-store.ts';

export type NotesCategoryRow = {
  id: string;
  title: string;
  description: string;
  /** The built-in inbox of this workspace. Title may change; this flag does not. */
  is_inbox: boolean;
  /** Id of the configured icon; empty = the default project icon. */
  icon: string;
  /** How many documents the project holds. */
  doc_count: number;
};

export type NotesCategoriesSnap = {
  /** Workspace whose projects the panel shows (the selected settings nav item). */
  workspaceId: string;
  rows: NotesCategoryRow[];
  selectedId: string;
  draftTitle: string;
  draftDescription: string;
  /** Icon chosen in the dialog; empty = the default icon. */
  draftIcon: string;
  error: string;
  busy: boolean;
  editorOpen: boolean;
};

export function emptyNotesCategoriesSnap(): NotesCategoriesSnap {
  return {
    workspaceId: '',
    rows: [],
    selectedId: '',
    draftTitle: '',
    draftDescription: '',
    draftIcon: '',
    error: '',
    busy: false,
    editorOpen: false,
  };
}

export const notesCategoriesStore = createModuleStore<NotesCategoriesSnap>(emptyNotesCategoriesSnap());

export function patchNotesCategories(partial: Partial<NotesCategoriesSnap>) {
  notesCategoriesStore.set({ ...notesCategoriesStore.getSnapshot(), ...partial });
}
