import { createModuleStore } from '../../../shared/module-store.ts';

export type NotesCategoryRow = {
  id: string;
  title: string;
  description: string;
  folder: string;
  /** Id of the configured icon; empty = the default project icon. */
  icon: string;
};

export type NotesCategoriesSnap = {
  /** Workspace whose projects the panel shows (the selected settings nav item). */
  workspaceId: string;
  rows: NotesCategoryRow[];
  /** Disabled projects of that workspace; they can be enabled again. */
  disabled: NotesCategoryRow[];
  selectedId: string;
  draftTitle: string;
  draftDescription: string;
  /** Icon chosen in the dialog; empty = the default icon. */
  draftIcon: string;
  error: string;
  busy: boolean;
  editorOpen: boolean;
  /** Project row waiting for "Confirm disable"; empty when none. */
  confirmDisableId: string;
};

export function emptyNotesCategoriesSnap(): NotesCategoriesSnap {
  return {
    workspaceId: '',
    rows: [],
    disabled: [],
    selectedId: '',
    draftTitle: '',
    draftDescription: '',
    draftIcon: '',
    error: '',
    busy: false,
    editorOpen: false,
    confirmDisableId: '',
  };
}

export const notesCategoriesStore = createModuleStore<NotesCategoriesSnap>(emptyNotesCategoriesSnap());

export function patchNotesCategories(partial: Partial<NotesCategoriesSnap>) {
  notesCategoriesStore.set({ ...notesCategoriesStore.getSnapshot(), ...partial });
}
