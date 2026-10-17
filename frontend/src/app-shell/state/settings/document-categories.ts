import { createModuleStore } from '../../../shared/module-store.ts';
import type { DocCategory } from '../../../host/api.ts';

export type DocumentCategoriesSnap = {
  rows: DocCategory[];
  newId: string;
  newLabel: string;
  newExposed: boolean;
  error: string;
  busy: boolean;
};

export function emptyDocumentCategoriesSnap(): DocumentCategoriesSnap {
  return {
    rows: [],
    newId: '',
    newLabel: '',
    newExposed: true,
    error: '',
    busy: false,
  };
}

export const documentCategoriesStore = createModuleStore<DocumentCategoriesSnap>(
  emptyDocumentCategoriesSnap(),
);

export function patchDocumentCategories(partial: Partial<DocumentCategoriesSnap>) {
  documentCategoriesStore.set({ ...documentCategoriesStore.getSnapshot(), ...partial });
}
