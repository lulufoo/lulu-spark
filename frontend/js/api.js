import {
  createApiClient,
  createFetchDriver,
  createTauriDriver,
  resolveReadDriver,
} from './apiClient.js';
import { getKbHidePattern } from './kb-hide-pattern.js';

function isTauriRuntime() {
  if (typeof window === 'undefined') return false;
  return Boolean(window.__TAURI__ || window.__TAURI_INTERNALS__);
}

function isLikelyExternalBrowserOnTauriDev() {
  if (typeof window === 'undefined') return false;
  const host = window.location.hostname;
  const isLocalDevHost = host === '127.0.0.1' || host === 'localhost';
  return isLocalDevHost && window.location.port === '1430' && !isTauriRuntime();
}

function normalizeReadError(error) {
  const message = error instanceof Error ? error.message : String(error);
  const isFetchFailure = /Failed to fetch|NetworkError/i.test(message);
  if (isLikelyExternalBrowserOnTauriDev() && isFetchFailure) {
    return new Error('检测到当前在外部浏览器打开了 Tauri Dev 页面，请回到 Tauri 应用窗口运行。');
  }
  return error instanceof Error ? error : new Error(message);
}

/** Resolve on each call: `tauri dev` loads page from :1430 before `__TAURI__` exists at import time. */
function getReadDriver() {
  const mode = resolveReadDriver();
  return resolveReadDriver(mode);
}

function getReadApi() {
  return createApiClient(getReadDriver());
}

function resolveWriteDriver() {
  const env =
    typeof import.meta !== 'undefined' && import.meta.env?.VITE_WRITE_API;
  if (env === 'fetch') return createFetchDriver();
  if (env === 'tauri') return createTauriDriver();
  return isTauriRuntime() ? createTauriDriver() : createFetchDriver();
}

function getWriteDriver() {
  return resolveWriteDriver();
}

async function writePost(path, body) {
  const res = await getWriteDriver().postJson(path, body);
  const payload = await res.json();
  return assertWritePayload(payload);
}

/** Tauri write commands return `{ error, _status }` without throwing — normalize here. */
export function assertWritePayload(payload) {
  if (payload && typeof payload === 'object' && payload.error) {
    const msg = typeof payload.error === 'string' ? payload.error : '请求失败';
    const err = new Error(msg);
    if (typeof payload._status === 'number') err.status = payload._status;
    throw err;
  }
  return payload;
}

/** Tauri read commands return `{ error, _status }` without throwing — normalize here. */
export function assertReadPayload(payload) {
  if (payload && typeof payload === 'object' && payload.error) {
    const msg = typeof payload.error === 'string' ? payload.error : '请求失败';
    const err = new Error(msg);
    if (typeof payload._status === 'number') err.status = payload._status;
    throw err;
  }
  return payload;
}

async function readGet(pathAndQuery) {
  try {
    const payload = await getReadApi().getJson(pathAndQuery);
    return assertReadPayload(payload);
  } catch (error) {
    throw normalizeReadError(error);
  }
}

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

export async function fetchKbFileContent(repo, path) {
  const res = await getReadDriver().fetchGet(
    `/api/kb/read?repo=${encodeURIComponent(repo)}&path=${encodeURIComponent(path)}`
  );
  if (!res.ok) {
    let msg = `HTTP ${res.status}`;
    try { const d = await res.json(); if (d.error) msg = d.error; } catch (_) {}
    throw new Error(msg);
  }
  return res.json();
}

export async function fetchKbList(repo, path = '', mode = 'flat') {
  const params = new URLSearchParams({ repo, path, mode });
  return readGet(`/api/kb/list?${params.toString()}`);
}

const KB_DOC_COUNT_TIMEOUT_MS = 10_000;

export async function fetchKbDocCount(repo, hidePattern) {
  const resolvedHide =
    hidePattern !== undefined ? hidePattern : getKbHidePattern();
  const params = new URLSearchParams({ repo });
  if (resolvedHide) {
    params.set('hide_pattern', resolvedHide);
  }
  const path = `/api/kb/doc-count?${params.toString()}`;

  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), KB_DOC_COUNT_TIMEOUT_MS);
  try {
    const res = await Promise.race([
      getReadDriver().fetchGet(path),
      new Promise((_, reject) => {
        controller.signal.addEventListener(
          'abort',
          () => {
            reject(controller.signal.reason ?? new DOMException('Aborted', 'AbortError'));
          },
          { once: true },
        );
      }),
    ]);
    if (!res.ok) {
      let msg = `HTTP ${res.status}`;
      try {
        const d = await res.json();
        if (d.error) msg = d.error;
      } catch (_) {}
      throw new Error(msg);
    }
    const data = await res.json();
    if (data && typeof data === 'object' && data.error) {
      throw new Error(typeof data.error === 'string' ? data.error : '请求失败');
    }
    return data?.count ?? 0;
  } catch (error) {
    throw normalizeReadError(error);
  } finally {
    clearTimeout(timeoutId);
  }
}

export async function fetchKbAnnotation(repo, path) {
  const res = await getReadDriver().fetchGet(
    `/api/kb/annotation?repo=${encodeURIComponent(repo)}&path=${encodeURIComponent(path)}`
  );
  if (!res.ok) return {};
  return res.json();
}

export async function updateKbComment(repo, path, comment, ts) {
  return writePost('/api/kb/update-comments', { repo, path, comment, ts });
}

export async function reorderKbComments(repo, path, ids) {
  return writePost('/api/kb/reorder-comments', { repo, path, ids });
}

export async function updateKbHighlight(repo, path, highlight, ts) {
  return writePost('/api/kb/update-highlights', { repo, path, highlight, ts });
}

export async function updateKbLinks(repo, path, links) {
  return writePost('/api/kb/update-links', { repo, path, links });
}

export async function saveKbFile(repo, path, content) {
  return writePost('/api/kb/save', { repo, path, content });
}

export async function commitKbFile(repo, message) {
  return writePost('/api/kb/commit', { repo, message });
}

export async function fetchKbStatus(repo) {
  const res = await getReadDriver().fetchGet(`/api/kb/status?repo=${encodeURIComponent(repo)}`);
  return res.json();
}

export async function fetchKbDiffStatus() {
  return readGet('/api/kb/diff-status?_=' + Date.now());
}

export async function revertKbFile(repo, path, type) {
  const body = { repo };
  if (path) { body.path = path; body.type = type; }
  return writePost('/api/kb/revert', body);
}

export async function reindexKbRepo(repo) {
  const { invokeSearch } = await import('./apiClient.js');
  return invokeSearch('reindexKbRepo', { repo });
}

export async function openItermAt(repo) {
  return writePost('/api/open-iterm', { repo });
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

export async function syncKnowledgeCorpus() {
  const { invokeSearch } = await import('./apiClient.js');
  return invokeSearch('syncKnowledgeCorpus');
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

export async function searchKnowledge(q, limit = 10) {
  const res = await getReadDriver().fetchGet(
    `/api/search-knowledge?q=${encodeURIComponent(q)}&limit=${limit}`
  );
  return res.json();
}

export async function reindexKnowledge() {
  const { invokeSearch } = await import('./apiClient.js');
  return invokeSearch('reindexKnowledge');
}

export async function getReindexStatus() {
  const { invokeSearch } = await import('./apiClient.js');
  return invokeSearch('getReindexStatus');
}

export async function searchWorkbench(q, limit = 10) {
  const res = await getReadDriver().fetchGet(
    `/api/search-workbench?q=${encodeURIComponent(q)}&limit=${limit}`
  );
  return res.json();
}

export async function reindexWorkbench() {
  const { invokeSearch } = await import('./apiClient.js');
  return invokeSearch('reindexWorkbench');
}

export async function getReindexWorkbenchStatus() {
  const { invokeSearch } = await import('./apiClient.js');
  return invokeSearch('getReindexWorkbenchStatus');
}

export async function fetchSedimentKbCategories() {
  return readGet('/api/sediment-kb/categories');
}

export async function fetchSedimentKbRepos() {
  return readGet('/api/sediment-kb/repos');
}

export async function addSedimentKbRepo(fullName, categoryId, description) {
  const body = { full_name: fullName };
  if (categoryId != null && categoryId !== '') {
    body.category_id = categoryId;
  }
  if (description != null && description !== '') {
    body.description = description;
  }
  return writePost('/api/sediment-kb/repos/add', body);
}

export async function removeSedimentKbRepo(fullName) {
  return writePost('/api/sediment-kb/repos/remove', { full_name: fullName });
}

export async function updateSedimentKbRepoCategory(fullName, categoryId) {
  return writePost('/api/sediment-kb/repos/update-category', {
    full_name: fullName,
    category_id: categoryId,
  });
}

export async function addSedimentKbCategory(name) {
  return writePost('/api/sediment-kb/categories/add', { name });
}

export async function renameSedimentKbCategory(id, name) {
  return writePost('/api/sediment-kb/categories/rename', { id, name });
}

export async function removeSedimentKbCategory(id) {
  return writePost('/api/sediment-kb/categories/remove', { id });
}
