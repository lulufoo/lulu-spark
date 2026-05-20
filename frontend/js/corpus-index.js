/**
 * Normalize workbench corpus `index.json` payload into mutable entry map.
 * @param {unknown} data
 * @returns {Record<string, object>}
 */
export function normalizeCorpusIndex(data) {
  if (!data || typeof data !== 'object' || Array.isArray(data)) {
    throw new Error('index.json 格式无效');
  }
  if (typeof data.error === 'string' && data.error) {
    throw new Error(data.error);
  }
  const raw = data.entries ?? data;
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) {
    throw new Error('index.json 缺少 entries');
  }
  if (typeof raw.error === 'string' && raw.error) {
    throw new Error(raw.error);
  }
  const normalized = {};
  for (const [id, entry] of Object.entries(raw)) {
    if (!entry || typeof entry !== 'object' || Array.isArray(entry)) continue;
    normalized[id] = { ...entry, _id: id };
  }
  return normalized;
}
