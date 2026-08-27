import { getKbHidePattern } from '../../corpus/corpus-hide-pattern.js';
import { getReadDriver, normalizeReadError, readGet, writePost } from './transport.js';

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
      throw new Error(typeof data.error === 'string' ? data.error : 'Request failed');
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
  const { invokeSearch } = await import('../apiClient.js');
  return invokeSearch('reindexKbRepo', { repo });
}

export async function openItermAt(repo) {
  return writePost('/api/open-iterm', { repo });
}

export async function syncKnowledgeCorpus() {
  const { invokeSearch } = await import('../apiClient.js');
  return invokeSearch('syncKnowledgeCorpus');
}

export async function searchKnowledge(q, limit = 10) {
  const res = await getReadDriver().fetchGet(
    `/api/search-knowledge?q=${encodeURIComponent(q)}&limit=${limit}`
  );
  return res.json();
}

export async function reindexKnowledge() {
  const { invokeSearch } = await import('../apiClient.js');
  return invokeSearch('reindexKnowledge');
}

export async function getReindexStatus() {
  const { invokeSearch } = await import('../apiClient.js');
  return invokeSearch('getReindexStatus');
}

export async function searchWorkbench(q, limit = 10) {
  const res = await getReadDriver().fetchGet(
    `/api/search-workbench?q=${encodeURIComponent(q)}&limit=${limit}`
  );
  return res.json();
}

export async function reindexWorkbench() {
  const { invokeSearch } = await import('../apiClient.js');
  return invokeSearch('reindexWorkbench');
}

export async function getReindexWorkbenchStatus() {
  const { invokeSearch } = await import('../apiClient.js');
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
