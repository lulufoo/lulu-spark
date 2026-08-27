import { getReadDriver, readGet, writePost } from './transport.js';

export async function fetchIndex() {
  return readGet('/api/corpus-index?_=' + Date.now());
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

export async function fetchAnnotation(path) {
  const res = await getReadDriver().fetchGet(`/api/annotation?path=${encodeURIComponent(path)}`);
  return res.json();
}

export async function fetchConfig() {
  return readGet('/api/config');
}

export async function inferGithubUserUrl(workbenchKnowledgeRoot) {
  return readGet(
    `/api/infer-github-user-url?path=${encodeURIComponent(workbenchKnowledgeRoot)}&_=${Date.now()}`,
  );
}

export async function checkWorkbenchKnowledgeRoot(workbenchKnowledgeRoot) {
  return readGet(
    `/api/check-workbench-root?path=${encodeURIComponent(workbenchKnowledgeRoot)}&_=${Date.now()}`,
  );
}

export async function setConfig(payload) {
  return writePost('/api/config', payload || {});
}

export async function fetchFileContent(layer, commonPath) {
  const data = await readGet(
    `/api/corpus-file?layer=${encodeURIComponent(layer)}&path=${encodeURIComponent(commonPath)}&_=${Date.now()}`
  );
  return typeof data === 'string' ? data : (data?.content ?? '');
}

/** Load corpus raster asset via invoke; returns blob: URL (caller may revoke). */
export async function fetchCorpusAssetAsBlobUrl(layer, baseCommonPath, href) {
  const data = await readGet(
    `/api/corpus-asset?layer=${encodeURIComponent(layer)}&base=${encodeURIComponent(baseCommonPath)}&href=${encodeURIComponent(href)}&_=${Date.now()}`
  );
  if (data && typeof data === 'object' && data.error) {
    throw new Error(String(data.error));
  }
  const mime = data?.mime_type || 'application/octet-stream';
  const b64 = data?.data_b64;
  if (!b64 || typeof b64 !== 'string') {
    throw new Error('Missing asset payload');
  }
  const binary = atob(b64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  const blob = new Blob([bytes], { type: mime });
  return URL.createObjectURL(blob);
}

export async function fetchLinkTitle(url) {
  return readGet(`/api/fetch-title?url=${encodeURIComponent(url)}`);
}

export async function saveFile(layer, commonPath, content) {
  return writePost('/api/save', { layer, common_path: commonPath, content });
}

// files 省略时提交全部变更；为数组时仅提交指定文件
export async function commitFiles(message, files) {
  const body = { message };
  if (files !== undefined) body.files = files;
  return writePost('/api/commit', body);
}

export async function revertFile(path, type) {
  return writePost('/api/corpus-revert', { path: path ?? '', type: type ?? '' });
}

export async function pullProject() {
  return writePost('/api/pull', {});
}

export async function updateComments(commonPath, layer, comment, ts) {
  return writePost('/api/update-comments', {
    common_path: commonPath,
    layer,
    comment,
    ts,
  });
}

export async function reorderComments(commonPath, layer, ids) {
  return writePost('/api/reorder-comments', {
    common_path: commonPath,
    layer,
    ids,
  });
}

export async function updateLinks(commonPath, links) {
  return writePost('/api/update-links', { common_path: commonPath, links });
}

export async function fetchTagsRegistry() {
  return readGet('/api/tags/registry');
}

export async function tagAttach(commonPath, payload) {
  return writePost('/api/tag/attach', {
    common_path: commonPath,
    ...payload,
  });
}

export async function tagDetach(commonPath, key) {
  return writePost('/api/tag/detach', { common_path: commonPath, key });
}

export async function tagUpdateValue(key, value) {
  return writePost('/api/tag/update-value', { key, value });
}

export async function setImportance(commonPath, importance) {
  return writePost('/api/set-importance', {
    common_path: commonPath,
    importance: importance || null,
  });
}

export async function setDone(commonPath, done) {
  return writePost('/api/set-done', { common_path: commonPath, done });
}

export async function deleteEntry(id) {
  return writePost('/api/delete', { id });
}

export async function ghMove(srcUrl, dstDirUrl) {
  return writePost('/api/gh-move', { src_url: srcUrl, dst_dir_url: dstDirUrl });
}

export async function ghDelete(url) {
  return writePost('/api/gh-delete', { url });
}

export async function updateHighlight(commonPath, layer, highlight, ts) {
  return writePost('/api/update-highlights', {
    common_path: commonPath,
    layer,
    highlight,
    ts,
  });
}

export async function fetchDocHighlights(key) {
  return readGet(`/api/doc-highlights?key=${encodeURIComponent(key)}`);
}

export async function updateDocHighlights(key, highlight, ts) {
  return writePost('/api/doc-highlights', { key, highlight, ts });
}

export async function fetchTopics() {
  const res = await getReadDriver().fetchGet('/api/topics?_=' + Date.now());
  const text = await res.text();
  let data;
  try {
    data = text ? JSON.parse(text) : {};
  } catch {
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    throw new Error('Invalid JSON response');
  }
  if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`);
  return data;
}

export async function getDraft(commonPath) {
  const res = await getReadDriver().fetchGet(`/api/draft?path=${encodeURIComponent(commonPath)}`);
  if (!res.ok) return { content: '' };
  return res.json();
}

export async function saveDraft(commonPath, content) {
  return writePost('/api/draft', { common_path: commonPath, content });
}

export async function moveToProject(id, newProject) {
  const data = await writePost('/api/move-project', { id, new_project: newProject });
  if (data && data.error) {
    throw new Error(data.error);
  }
  return data;
}

export async function fetchRepoDirs(repo) {
  return readGet(`/api/repo-dirs?repo=${encodeURIComponent(repo)}`);
}

export async function checkFileExists(repo, path) {
  return readGet(
    `/api/check-file?repo=${encodeURIComponent(repo)}&path=${encodeURIComponent(path)}`
  );
}

export async function settleComment(commonPath, commentId, layer, docTheme, slug, content) {
  return writePost('/api/settle', {
    common_path: commonPath,
    comment_id: commentId,
    layer,
    doc_theme: docTheme,
    slug,
    content,
  });
}

/** Archive via HTTP/MCP `archive_document` parity.
 * Note create: `{ body, source_type: 'note' }` (Host synthesize).
 * Path archive: `{ source_path, source_type }` — no `document` body. */
export async function archiveDocument(payload) {
  return writePost('/api/archive-document', payload || {});
}

/** Note-create crash buffer under `drafts/notes/<temp_id>`. */
export async function saveNoteDraft(tempId, content) {
  return writePost('/api/note-draft', { temp_id: tempId, content: content ?? '' });
}

export async function clearNoteDraft(tempId) {
  return writePost('/api/note-draft/clear', { temp_id: tempId });
}

export async function getNoteDraft(tempId) {
  const res = await getReadDriver().fetchGet(
    `/api/note-draft?temp_id=${encodeURIComponent(tempId)}`,
  );
  if (!res.ok) return { content: '' };
  return res.json();
}
