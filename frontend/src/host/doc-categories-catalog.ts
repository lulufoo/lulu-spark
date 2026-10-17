import { createModuleStore } from '../shared/module-store.ts';
import { fetchDocCategories, type DocCategory } from './api/doc-categories.ts';

export type DocCategoryCatalog = { items: DocCategory[] };

export function emptyDocCategoryCatalog(): DocCategoryCatalog {
  return { items: [] };
}

export const docCategoryCatalogStore = createModuleStore<DocCategoryCatalog>(emptyDocCategoryCatalog());

export function setDocCategoryCatalog(items: DocCategory[]) {
  docCategoryCatalogStore.set({ items });
}

export function categoryLabel(id?: string): string | undefined {
  if (!id) return undefined;
  return docCategoryCatalogStore.getSnapshot().items.find((item) => item.id === id)?.label;
}

export function exposedCategoryOptions(): { value: string; label: string }[] {
  return docCategoryCatalogStore
    .getSnapshot()
    .items.filter((item) => item.exposed)
    .map((item) => ({ value: item.id, label: item.label }));
}

/** Load the local classification list for badges and pickers. */
export async function loadDocCategoryCatalog() {
  try {
    setDocCategoryCatalog(await fetchDocCategories());
  } catch {
    setDocCategoryCatalog([]);
  }
}
