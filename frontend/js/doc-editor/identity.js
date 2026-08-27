/** Canonical highlight identity. Host MD5s this string for the cache filename. */

export function notesDocKey(commonPath) {
  return `notes:${String(commonPath || '').trim()}`;
}

export function knowledgeDocKey(repo, relativePath) {
  return `knowledge:${String(repo || '').trim()}/${String(relativePath || '').trim()}`;
}

export function todosDocKey(taskId, extra) {
  const id = String(taskId || '').trim();
  const suffix = extra != null && String(extra).trim() ? `:${String(extra).trim()}` : '';
  return `todos:${id}${suffix}`;
}
