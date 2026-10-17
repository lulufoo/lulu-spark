/** Canonical highlight identity. Host MD5s this string for the cache filename. */

export function docKey(id: unknown): string {
  return `doc:${String(id || '').trim()}`;
}
