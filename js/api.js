export async function fetchIndex() {
  const res = await fetch('./index.json?_=' + Date.now());
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function fetchDiffStatus() {
  const res = await fetch('/api/status?_=' + Date.now());
  if (!res.ok) return null;
  return res.json();
}

export async function fetchAnnotationsSummary() {
  const res = await fetch('/api/annotations?_=' + Date.now());
  if (!res.ok) return null;
  return res.json();
}

export async function fetchAnnotation(path) {
  const res = await fetch(`/api/annotation?path=${encodeURIComponent(path)}`);
  return res.json();
}

export async function fetchConfig() {
  const res = await fetch('/api/config');
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
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
  const res = await fetch('./topics.json?_=' + Date.now());
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function updateTopics(mode = 'fast', checkRepo = '') {
  const body = { mode };
  if (checkRepo) body.check_repo = checkRepo;
  const res = await fetch('/api/update-topics', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body)
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
  const res = await fetch(`/api/repo-dirs?repo=${encodeURIComponent(repo)}`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function fetchRepoList() {
  const res = await fetch('/api/repo-list');
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function checkFileExists(repo, path) {
  const res = await fetch(`/api/check-file?repo=${encodeURIComponent(repo)}&path=${encodeURIComponent(path)}`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
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
