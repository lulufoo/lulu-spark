/**
 * Normalize workbench corpus `index.json` payload into mutable entry map.
 */
export function normalizeCorpusIndex(data: unknown): Record<string, object> {
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
