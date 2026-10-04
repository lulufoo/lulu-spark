import { asRecord, type PathArg } from '../api-types.ts';
import { getReadDriver, readGet, writePost } from './transport.ts';

export async function fetchIndex() {
  return readGet('/api/notes-index?_=' + Date.now());
}

export async function fetchDiffStatus() {
  return readGet('/api/status?_=' + Date.now());
}

export async function fetchAnnotationsSummary() {
  try {
    return await readGet('/api/annotations?_=' + Date.now());
  } catch {
    return null;
  }
}

export async function fetchAnnotation(path: PathArg) {
  const res = await getReadDriver().fetchGet(
    `/api/annotation?path=${encodeURIComponent(String(path ?? ''))}`,
  );
  return res.json();
}

export async function fetchConfig() {
  return readGet('/api/config');
}

export async function inferGithubUserUrl(sparkRoot: string) {
  return readGet(
    `/api/infer-github-user-url?path=${encodeURIComponent(sparkRoot)}&_=${Date.now()}`,
  );
}

export async function checkSparkRoot(sparkRoot: string) {
  return readGet(
    `/api/check-spark-root?path=${encodeURIComponent(sparkRoot)}&_=${Date.now()}`,
  );
}

export async function setConfig(payload?: unknown) {
  return writePost('/api/config', payload || {});
}

export async function fetchFileContent(layer: PathArg, commonPath: PathArg): Promise<string> {
  const data = await readGet(
    `/api/notes-file?layer=${encodeURIComponent(String(layer ?? ''))}&path=${encodeURIComponent(String(commonPath ?? ''))}&_=${Date.now()}`
  );
  if (typeof data === 'string') return data;
  const content = asRecord(data)?.content;
  return typeof content === 'string' ? content : '';
}

/** Load notes raster asset via invoke; returns blob: URL (caller may revoke). */
export async function fetchNotesAssetAsBlobUrl(
  layer: string,
  baseCommonPath: string,
  href: string,
) {
  const data = await readGet(
    `/api/notes-asset?layer=${encodeURIComponent(layer)}&base=${encodeURIComponent(baseCommonPath)}&href=${encodeURIComponent(href)}&_=${Date.now()}`
  );
  const rec = asRecord(data);
  if (rec?.error) {
    throw new Error(String(rec.error));
  }
  const mime = rec?.mime_type || 'application/octet-stream';
  const b64 = rec?.data_b64;
  if (!b64 || typeof b64 !== 'string') {
    throw new Error('Missing asset payload');
  }
  const binary = atob(b64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  const blob = new Blob([bytes], { type: String(mime) });
  return URL.createObjectURL(blob);
}

export async function fetchLinkTitle(url: string) {
  return readGet(`/api/fetch-title?url=${encodeURIComponent(url)}`);
}

export async function saveFile(layer: PathArg, commonPath: PathArg, content: string) {
  return writePost('/api/save', { layer, common_path: commonPath, content });
}

// files 省略时提交全部变更；为数组时仅提交指定文件
export async function commitFiles(message: string, files?: unknown) {
  const body: Record<string, unknown> = { message };
  if (files !== undefined) body.files = files;
  return writePost('/api/commit', body);
}

export async function revertFile(path?: string, type?: string) {
  return writePost('/api/spark-revert', { path: path ?? '', type: type ?? '' });
}

export async function pullProject() {
  return writePost('/api/pull', {});
}

export async function updateComments(
  commonPath: PathArg,
  layer: PathArg,
  comment: unknown,
  ts?: unknown,
) {
  return writePost('/api/update-comments', {
    common_path: commonPath,
    layer,
    comment,
    ts,
  });
}

export async function reorderComments(commonPath: string, layer: string, ids: unknown) {
  return writePost('/api/reorder-comments', {
    common_path: commonPath,
    layer,
    ids,
  });
}

export async function updateLinks(commonPath: string, links: unknown) {
  return writePost('/api/update-links', { common_path: commonPath, links });
}

export async function fetchTagsRegistry() {
  return readGet('/api/tags/registry');
}

export async function tagAttach(commonPath: string, payload?: Record<string, unknown>) {
  return writePost('/api/tag/attach', {
    common_path: commonPath,
    ...payload,
  });
}

export async function tagDetach(commonPath: string, key: string) {
  return writePost('/api/tag/detach', { common_path: commonPath, key });
}

export async function tagUpdateValue(key: string, value: unknown) {
  return writePost('/api/tag/update-value', { key, value });
}

export async function setImportance(commonPath: string, importance?: unknown) {
  return writePost('/api/set-importance', {
    common_path: commonPath,
    importance: importance || null,
  });
}

export async function setDone(commonPath: string, done: unknown) {
  return writePost('/api/set-done', { common_path: commonPath, done });
}

export async function deleteEntry(id: PathArg) {
  return writePost('/api/delete', { id });
}

export async function ghDelete(url: string) {
  return writePost('/api/gh-delete', { url });
}

export async function updateHighlight(
  commonPath: string,
  layer: string,
  highlight: unknown,
  ts?: unknown,
) {
  return writePost('/api/update-highlights', {
    common_path: commonPath,
    layer,
    highlight,
    ts,
  });
}

export async function fetchDocHighlights(key: string) {
  return readGet(`/api/doc-highlights?key=${encodeURIComponent(key)}`);
}

export async function updateDocHighlights(key: string, highlight: unknown, ts?: unknown) {
  return writePost('/api/doc-highlights', { key, highlight, ts });
}

export async function fetchTopics() {
  const res = await getReadDriver().fetchGet('/api/topics?_=' + Date.now());
  const text = await res.text();
  let data: unknown;
  try {
    data = text ? JSON.parse(text) : {};
  } catch {
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    throw new Error('Invalid JSON response');
  }
  const rec = asRecord(data);
  if (!res.ok) throw new Error(String(rec?.error || `HTTP ${res.status}`));
  return data;
}

export async function getDraft(commonPath: string) {
  const res = await getReadDriver().fetchGet(`/api/draft?path=${encodeURIComponent(commonPath)}`);
  if (!res.ok) return { content: '' };
  return res.json();
}

export async function saveDraft(commonPath: PathArg, content: string) {
  return writePost('/api/draft', { common_path: commonPath, content });
}

export async function moveToProject(id: string, newProject: string) {
  const data = await writePost('/api/move-project', { id, new_project: newProject });
  const rec = asRecord(data);
  if (rec?.error) {
    throw new Error(String(rec.error));
  }
  return data;
}

export async function fetchRepoDirs(repo: string) {
  return readGet(`/api/repo-dirs?repo=${encodeURIComponent(repo)}`);
}

export async function checkFileExists(repo: string, path: string) {
  return readGet(
    `/api/check-file?repo=${encodeURIComponent(repo)}&path=${encodeURIComponent(path)}`
  );
}

export async function settleComment(
  commonPath: PathArg,
  commentId: PathArg,
  layer: PathArg,
  docTheme: unknown,
  slug: PathArg,
  content: string,
  repo: string,
) {
  return writePost('/api/settle', {
    common_path: commonPath,
    comment_id: commentId,
    layer,
    doc_theme: docTheme,
    slug,
    content,
    repo,
  });
}

/** Create a note via HTTP/MCP `create_note` parity.
 * Jot create: `{ body, source_type: 'jot' }` (Host synthesize; digest=never).
 * Path create: `{ source_path, title, digest, asset_paths? }` — no `document` body. */
export async function createNote(payload?: unknown) {
  return writePost('/api/create-note', payload || {});
}

/** Note-create crash buffer under `drafts/notes/<temp_id>`. */
export async function saveNoteDraft(tempId: string, content?: string) {
  return writePost('/api/note-draft', { temp_id: tempId, content: content ?? '' });
}

export async function clearNoteDraft(tempId: string) {
  return writePost('/api/note-draft/clear', { temp_id: tempId });
}

export async function getNoteDraft(tempId: string) {
  const res = await getReadDriver().fetchGet(
    `/api/note-draft?temp_id=${encodeURIComponent(tempId)}`,
  );
  if (!res.ok) return { content: '' };
  return res.json();
}
