import * as api from '../../../host/api.ts';
import type { DocCategory } from '../../../host/api.ts';
import { setDocCategoryCatalog } from '../../../host/doc-categories-catalog.ts';
import { errMessage } from '../../state/types.ts';
import {
  documentCategoriesStore,
  patchDocumentCategories,
} from '../../state/settings/document-categories.ts';

const ID_RE = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

/** Lowercase hyphen id from a display name: "Theme line" → "theme-line". */
export function slugFromLabel(label: string): string {
  return label
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

export function validateCategoryId(id: string): string | null {
  if (id.trim() === '') return 'Could not make an id from this name. Use English letters or digits.';
  if (/\s/.test(id)) return 'Category id cannot contain spaces.';
  if (!ID_RE.test(id.trim())) return 'Use lowercase letters, digits, and hyphens.';
  return null;
}

function describeError(err: unknown): string {
  return errMessage(err, String(err));
}

export function setNewCategoryLabel(value: string) {
  patchDocumentCategories({ newLabel: value, newId: slugFromLabel(value), error: '' });
}

export function setNewCategoryExposed(value: boolean) {
  patchDocumentCategories({ newExposed: value, error: '' });
}

export function setCategoryLabelDraft(id: string, label: string) {
  const rows = documentCategoriesStore.getSnapshot().rows.map((row) =>
    row.id === id ? { ...row, label } : row,
  );
  patchDocumentCategories({ rows, error: '' });
}

export async function loadDocCategories() {
  patchDocumentCategories({ error: '', busy: true });
  try {
    const rows = await api.fetchDocCategories();
    setDocCategoryCatalog(rows);
    patchDocumentCategories({ rows, busy: false });
  } catch (err) {
    patchDocumentCategories({ error: describeError(err), busy: false });
  }
}

async function persist(next: DocCategory[]) {
  patchDocumentCategories({ busy: true, error: '' });
  try {
    const rows = await api.saveDocCategories(next);
    setDocCategoryCatalog(rows);
    patchDocumentCategories({ rows, busy: false });
    return true;
  } catch (err) {
    patchDocumentCategories({ error: describeError(err), busy: false });
    try {
      const rows = await api.fetchDocCategories();
      setDocCategoryCatalog(rows);
      patchDocumentCategories({ rows });
    } catch {
      /* keep the failed write's rows so the person can retry */
    }
    return false;
  }
}

export async function toggleCategoryExposed(id: string, exposed: boolean) {
  const rows = documentCategoriesStore.getSnapshot().rows.map((row) =>
    row.id === id ? { ...row, exposed } : row,
  );
  patchDocumentCategories({ rows });
  await persist(rows);
}

export async function saveCategoryLabel(id: string) {
  const row = documentCategoriesStore.getSnapshot().rows.find((item) => item.id === id);
  if (!row) return;
  const label = row.label.trim();
  if (!label) {
    patchDocumentCategories({ error: 'Category label is empty.' });
    await loadDocCategories();
    return;
  }
  const rows = documentCategoriesStore.getSnapshot().rows.map((item) =>
    item.id === id ? { ...item, label } : item,
  );
  await persist(rows);
}

export async function addDocCategory() {
  const snap = documentCategoriesStore.getSnapshot();
  const label = snap.newLabel.trim();
  if (!label) {
    patchDocumentCategories({ error: 'Category label is empty.' });
    return false;
  }
  const id = slugFromLabel(label);
  const idError = validateCategoryId(id);
  if (idError) {
    patchDocumentCategories({ error: idError });
    return false;
  }
  if (snap.rows.some((row) => row.id === id)) {
    patchDocumentCategories({ error: `duplicate category id: ${id}` });
    return false;
  }
  const next = [...snap.rows, { id, label, exposed: snap.newExposed }];
  const ok = await persist(next);
  if (ok) patchDocumentCategories({ newId: '', newLabel: '', newExposed: true });
  return ok;
}

export async function removeDocCategory(id: string) {
  const next = documentCategoriesStore.getSnapshot().rows.filter((row) => row.id !== id);
  return persist(next);
}
