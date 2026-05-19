import { createApiClient, createFetchDriver } from './apiClient.js';

const readDriver = createFetchDriver();
const readApi = createApiClient(readDriver);

async function readGet(pathAndQuery) {
  return readApi.getJson(pathAndQuery);
}

export async function fetchIndex() {
  const res = await fetch('./index.json?_=' + Date.now());
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function fetchDiffStatus() {
  try {
    return await readGet('/api/status?_=' + Date.now());
  } catch {
    return null;
  }
}

export async function fetchAnnotationsSummary() {
  try {
    return await readGet('/api/annotations?_=' + Date.now());
  } catch {
    return null;
  }
}

export async function fetchAnnotation(path) {
  const res = await readDriver.fetchGet(`/api/annotation?path=${encodeURIComponent(path)}`);
  return res.json();
}

export async function fetchConfig() {
  return readGet('/api/config');
}

export async function fetchFileContent(layer, commonPath) {
  const res = await fetch(`./${layer}/${commonPath}?t=${Date.now()}`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.text();
}

export async function fetchLinkTitle(url) {
  const res = await fetch(`/api/fetch-title?url=${encodeURIComponent(url)}`);
  return res.json();
}

export async function saveFile(layer, commonPath, content) {
  const res = await fetch('/api/save', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ layer, common_path: commonPath, content })
  });
  return res.json();
}

// files 省略时提交全部变更；为数组时仅提交指定文件
export async function commitFiles(message, files) {
  const body = { message };
  if (files !== undefined) body.files = files;
  const res = await fetch('/api/commit', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body)
  });
  return res.json();
}

export async function fetchKbFileContent(repo, path) {
  const res = await readDriver.fetchGet(
    `/api/kb/read?repo=${encodeURIComponent(repo)}&path=${encodeURIComponent(path)}`
  );
  if (!res.ok) {
    let msg = `HTTP ${res.status}`;
    try { const d = await res.json(); if (d.error) msg = d.error; } catch (_) {}
    throw new Error(msg);
  }
  return res.json();
}

export async function fetchKbAnnotation(repo, path) {
  const res = await readDriver.fetchGet(
    `/api/kb/annotation?repo=${encodeURIComponent(repo)}&path=${encodeURIComponent(path)}`
  );
  if (!res.ok) return {};
  return res.json();
}

export async function updateKbComment(repo, path, comment, ts) {
  const res = await fetch('/api/kb/update-comments', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ repo, path, comment, ts })
  });
  return res.json();
}

export async function reorderKbComments(repo, path, ids) {
  const res = await fetch('/api/kb/reorder-comments', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ repo, path, ids })
  });
  return res.json();
}

export async function updateKbHighlight(repo, path, highlight, ts) {
  const res = await fetch('/api/kb/update-highlights', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ repo, path, highlight, ts })
  });
  return res.json();
}

export async function updateKbLinks(repo, path, links) {
  const res = await fetch('/api/kb/update-links', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ repo, path, links })
  });
  return res.json();
}

export async function saveKbFile(repo, path, content) {
  const res = await fetch('/api/kb/save', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ repo, path, content })
  });
  return res.json();
}

export async function commitKbFile(repo, message) {
  const res = await fetch('/api/kb/commit', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ repo, message })
  });
  return res.json();
}

export async function fetchKbStatus(repo) {
  const res = await readDriver.fetchGet(`/api/kb/status?repo=${encodeURIComponent(repo)}`);
  return res.json();
}

export async function revertKbFile(repo, path, type) {
  const body = { repo };
  if (path) { body.path = path; body.type = type; }
  const res = await fetch('/api/kb/revert', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body)
  });
  return res.json();
}

export async function reindexKbRepo(repo) {
  const res = await fetch('/api/kb/reindex', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ repo })
  });
  return res.json();
}

export async function openItermAt(repo) {
  const res = await fetch('/api/open-iterm', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ repo })
  });
  return res.json();
}

export async function pullProject() {
  const res = await fetch('/api/pull', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({})
  });
  return res.json();
}

export async function updateComments(commonPath, layer, comment, ts) {
  const res = await fetch('/api/update-comments', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ common_path: commonPath, layer, comment, ts })
  });
  return res.json();
}

export async function reorderComments(commonPath, layer, ids) {
  const res = await fetch('/api/reorder-comments', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ common_path: commonPath, layer, ids })
  });
  return res.json();
}

export async function updateLinks(commonPath, links) {
  const res = await fetch('/api/update-links', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ common_path: commonPath, links })
  });
  return res.json();
}

export async function setImportance(commonPath, importance) {
  const res = await fetch('/api/set-importance', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ common_path: commonPath, importance: importance || null })
  });
  return res.json();
}

export async function setDone(commonPath, done) {
  const res = await fetch('/api/set-done', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ common_path: commonPath, done })
  });
  return res.json();
}

export async function deleteEntry(id) {
  const res = await fetch('/api/delete', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ id })
  });
  return res.json();
}

export async function ghMove(srcUrl, dstDirUrl) {
  const res = await fetch('/api/gh-move', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ src_url: srcUrl, dst_dir_url: dstDirUrl })
  });
  return res.json();
}

export async function updateHighlight(commonPath, layer, highlight, ts) {
  const res = await fetch('/api/update-highlights', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ common_path: commonPath, layer, highlight, ts })
  });
  return res.json();
}

export async function fetchTopics() {
  const res = await readDriver.fetchGet('/api/topics?_=' + Date.now());
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

export async function fetchKnowledgeIndex(force = false) {
  const path = force ? '/api/knowledge-index?force=1' : '/api/knowledge-index';
  const res = await readDriver.fetchGet(path);
  const data = await res.json();
  if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`);
  try {
    localStorage.setItem('lulu_wb_knowledge_index_cache', JSON.stringify(data));
  } catch (_) {}
  return data;
}

export async function updateTopics() {
  const res = await fetch('/api/update-topics', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: '{}',
  });
  return res.json();
}

export async function getDraft(commonPath) {
  const res = await readDriver.fetchGet(`/api/draft?path=${encodeURIComponent(commonPath)}`);
  if (!res.ok) return { content: '' };
  return res.json();
}

export async function saveDraft(commonPath, content) {
  const res = await fetch('/api/draft', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ common_path: commonPath, content })
  });
  return res.json();
}

export async function moveToProject(id, newProject) {
  const res = await fetch('/api/move-project', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ id, new_project: newProject })
  });
  return res.json();
}

export async function fetchRepoDirs(repo) {
  return readGet(`/api/repo-dirs?repo=${encodeURIComponent(repo)}`);
}

export async function fetchRepoList(force = false) {
  const path = force ? '/api/repo-list?force=1' : '/api/repo-list';
  return readGet(path);
}

export async function checkFileExists(repo, path) {
  return readGet(
    `/api/check-file?repo=${encodeURIComponent(repo)}&path=${encodeURIComponent(path)}`
  );
}

export async function settleComment(commonPath, commentId, layer, docTheme, slug, content) {
  const res = await fetch('/api/settle', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      common_path: commonPath,
      comment_id: commentId,
      layer,
      doc_theme: docTheme,
      slug,
      content
    })
  });
  return res.json();
}

export async function searchKnowledge(q, limit = 10) {
  const res = await readDriver.fetchGet(
    `/api/search-knowledge?q=${encodeURIComponent(q)}&limit=${limit}`
  );
  return res.json();
}

export async function reindexKnowledge() {
  const res = await fetch('/api/reindex-knowledge', { method: 'POST' });
  return res.json();
}

export async function getReindexStatus() {
  const res = await readDriver.fetchGet('/api/reindex-status');
  return res.json();
}

export async function searchWorkbench(q, limit = 10) {
  const res = await readDriver.fetchGet(
    `/api/search-workbench?q=${encodeURIComponent(q)}&limit=${limit}`
  );
  return res.json();
}

export async function reindexWorkbench() {
  const res = await fetch('/api/reindex-workbench', { method: 'POST' });
  return res.json();
}

export async function getReindexWorkbenchStatus() {
  const res = await readDriver.fetchGet('/api/reindex-workbench-status');
  return res.json();
}
