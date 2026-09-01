import { readGet, writePost } from './transport.ts';

export async function fetchNotesCategories() {
  return readGet('/api/notes-categories?_=' + Date.now());
}

export async function createNotesCategory(title: string, description = '') {
  return writePost('/api/notes-category-create', { title, description });
}

export async function updateNotesCategory(id: string, title: string, description = '') {
  return writePost('/api/notes-category-update', { id, title, description });
}

export async function deleteNotesCategory(id: string) {
  return writePost('/api/notes-category-delete', { id });
}
