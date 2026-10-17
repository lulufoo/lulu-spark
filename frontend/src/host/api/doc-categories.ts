import { invoke } from './transport.ts';

export type DocCategory = {
  id: string;
  label: string;
  exposed: boolean;
};

function asCategories(data: unknown): DocCategory[] {
  const rec = data as { categories?: unknown; error?: string } | null;
  if (rec?.error) throw new Error(rec.error);
  const rows = Array.isArray(rec?.categories) ? rec.categories : [];
  const out: DocCategory[] = [];
  for (const row of rows) {
    const item = row as { id?: unknown; label?: unknown; exposed?: unknown };
    if (typeof item.id !== 'string' || !item.id) continue;
    out.push({
      id: item.id,
      label: typeof item.label === 'string' ? item.label : item.id,
      exposed: item.exposed === true,
    });
  }
  return out;
}

/** Full classification list, including items that MCP does not expose. */
export async function fetchDocCategories(): Promise<DocCategory[]> {
  return asCategories(await invoke('get_doc_categories'));
}

/** Replace the local classification list. */
export async function saveDocCategories(categories: DocCategory[]): Promise<DocCategory[]> {
  return asCategories(await invoke('set_doc_categories', { categories }));
}
