/** Canonical highlight identity. Host MD5s this string for the cache filename. */

export function notesDocKey(commonPath: unknown): string {
  return `notes:${String(commonPath || '').trim()}`;
}
