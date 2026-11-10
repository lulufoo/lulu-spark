/**
 * Normalize the documents index payload into mutable entry map.
 */
export function normalizeDocIndex(data: unknown): Record<string, object> {
  if (!data || typeof data !== 'object' || Array.isArray(data)) {
    throw new Error('Invalid index.json format');
  }
  const record = data as Record<string, unknown>;
  if (typeof record.error === 'string' && record.error) {
    throw new Error(record.error);
  }
  const raw = record.entries ?? record;
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) {
    throw new Error('index.json missing entries');
  }
  const rawRecord = raw as Record<string, unknown>;
  if (typeof rawRecord.error === 'string' && rawRecord.error) {
    throw new Error(rawRecord.error);
  }
  const normalized: Record<string, object> = {};
  for (const [id, entry] of Object.entries(rawRecord)) {
    if (!entry || typeof entry !== 'object' || Array.isArray(entry)) continue;
    normalized[id] = { ...entry, _id: id };
  }
  return normalized;
}

/** Empty directories carried beside `entries` on the notes index. */
export function folderRefs(data: unknown): { project_id: string; path: string }[] {
  if (!data || typeof data !== 'object' || Array.isArray(data)) return [];
  const raw = (data as { folders?: unknown }).folders;
  if (!Array.isArray(raw)) return [];
  return raw.flatMap((item) => {
    if (!item || typeof item !== 'object' || Array.isArray(item)) return [];
    const row = item as { project_id?: unknown; path?: unknown };
    if (typeof row.project_id !== 'string' || typeof row.path !== 'string') return [];
    if (!row.project_id || !row.path) return [];
    return [{ project_id: row.project_id, path: row.path }];
  });
}
