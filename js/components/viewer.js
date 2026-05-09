import { state, getEntryId, loadDiffStatus } from '../state.js'
import { REPO } from '../constants.js'
import { escHtml, filenameFromPath, slugToTitle } from '../utils.js'
import * as api from '../api.js'
import { updateTitlesInDOM, updateDiffInDOM } from './cards.js'
import { renderLinksBar } from './links-bar.js'
import { renderComments } from './comments.js'
import { openDeleteDialog } from './modals/delete-dialog.js'
import { applyHighlights, initHighlightUI } from './highlights.js'
import { mountKnowledgeSearch, triggerKnowledgeSearch } from './knowledge-search.js'

// ── resolveRelativeLink ────────────────────────────────────────────────────

function resolveRelativeLink(href, layer, commonPath) {
  try {
    const base = `http://x/${layer}/${commonPath}`;
    const resolved = new URL(href, base);
    const repoPath = resolved.pathname.slice(1);
    return `${REPO}/${repoPath}`;
  } catch {
    return null;
  }
}

// ── postProcessLinks ───────────────────────────────────────────────────────

function postProcessLinks(container, layer, commonPath) {
  container.querySelectorAll('a[href]').forEach(a => {
    const href = a.getAttribute('href');
    if (!href || href.startsWith('#')) return;
    if (href.startsWith('http://') || href.startsWith('https://')) {
      a.target = '_blank';
      a.rel = 'noopener noreferrer';
      return;
    }
    const ghUrl = resolveRelativeLink(href, layer, commonPath);
    if (ghUrl) {
      a.href = ghUrl;
      a.target = '_blank';
      a.rel = 'noopener noreferrer';
    }
  });
}

// ── renderDocBody ──────────────────────────────────────────────────────────

export function renderDocBody(text, layer, commonPath) {
  const body = document.getElementById('md-body');
  if (typeof marked !== 'undefined') {
    body.innerHTML = marked.parse(text);
  } else {
    body.innerHTML = `<pre style="white-space:pre-wrap;font-size:13px">${escHtml(text)}</pre>`;
  }
  postProcessLinks(body, layer, commonPath);
  document.getElementById('btn-edit').style.display = '';
  renderLinksBar(state.viewer.entry);
  renderComments(state.viewer.annotation, layer, state.viewer.entry);
  const zone = document.createElement('div');
  zone.className = 'md-body-delete-zone';
  const delBtn = document.createElement('button');
  delBtn.id = 'btn-delete';
  delBtn.textContent = '🗑 删除此条目';
  delBtn.title = '删除此条目的所有关联文件（raw / distilled / trace / digest / diagnose）';
  delBtn.addEventListener('click', () => openDeleteDialog());
  zone.appendChild(delBtn);
  body.appendChild(zone);
}

// ── Language helpers ───────────────────────────────────────────────────────

function getActivePath(entry, lang) {
  if (lang === 'zh' && entry.translations?.zh) return entry.translations.zh;
  return entry.common_path;
}

function updateLangBar(entry) {
  const bar = document.getElementById('md-lang-bar');
  const hasZh = !!entry.translations?.zh;
  bar.style.display = hasZh ? 'flex' : 'none';
  if (hasZh) {
    document.getElementById('btn-lang-en').classList.toggle('active', state.viewer.lang !== 'zh');
    document.getElementById('btn-lang-zh').classList.toggle('active', state.viewer.lang === 'zh');
  }
}

function updateHeaderUrls(entry, layer, activePath) {
  const githubUrl = `${REPO}/${layer}/${activePath}`;
  document.getElementById('md-github-link').href = githubUrl;
  document.getElementById('btn-copy-http').dataset.url = githubUrl;
  document.getElementById('btn-copy-http').dataset.tip = githubUrl;
  const relPath = `${layer}/${activePath}`;
  const fullPath = state.ui.archiveRoot ? `${state.ui.archiveRoot}/${relPath}` : relPath;
  document.getElementById('btn-copy-path').dataset.tip = fullPath;

  const topicDir = entry.common_path.split('/')[0];
  const kbUrl = state.index.topicRepos[topicDir];
  const kbBtn = document.getElementById('btn-goto-kb');
  if (kbUrl) {
    kbBtn.style.display = '';
    kbBtn.onclick = () => window.open(kbUrl, '_blank', 'noopener,noreferrer');
  } else {
    kbBtn.style.display = 'none';
  }
}

// ── openDoc ────────────────────────────────────────────────────────────────

export async function openDoc(entry, layer = 'raw') {
  state.viewer.entry = entry;
  state.viewer.layer = layer;
  state.viewer.annotation = {};
  state.viewer.lang = entry.translations?.zh ? 'zh' : null;
  exitEditMode(false);
  hideCommitBar();

  const modal = document.getElementById('md-modal');
  const body = document.getElementById('md-body');
  document.getElementById('md-panel-title').textContent = filenameFromPath(entry.common_path).replace(/\.md$/, '');
  const activePath = getActivePath(entry, state.viewer.lang);
  updateHeaderUrls(entry, layer, activePath);
  updateLangBar(entry);
  document.getElementById('md-file-size').textContent = '';
  body.innerHTML = '<div style="color:#8c959f;padding:20px;font-size:13px;">加载中…</div>';
  modal.style.display = 'flex';
  document.body.style.overflow = 'hidden';

  const [mdResult, annResult] = await Promise.allSettled([
    api.fetchFileContent(layer, activePath),
    api.fetchAnnotation(entry.common_path).catch(() => ({}))
  ]);

  if (annResult.status === 'fulfilled') state.viewer.annotation = annResult.value || {};

  if (mdResult.status === 'rejected') {
    body.innerHTML = `<div style="color:#7d4e00;padding:20px">无法加载文件：${escHtml(mdResult.reason.message)}</div>`;
    document.getElementById('btn-edit').style.display = 'none';
    return;
  }

  const text = mdResult.value;
  state.viewer.rawText = text;
  const bytes = new Blob([text]).size;
  document.getElementById('md-file-size').textContent = bytes < 1024
    ? `${bytes} B`
    : bytes < 1024 * 1024
      ? `${(bytes / 1024).toFixed(1)} KB`
      : `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  renderDocBody(text, layer, activePath);
  applyHighlights(state.viewer.annotation, layer);
  const hasDiff = state.index.diffStatus.get(`${layer}/${entry.common_path}`);
  document.getElementById('btn-panel-commit').style.display = hasDiff ? '' : 'none';

  mountKnowledgeSearch(document.getElementById('knowledge-panel'));
  triggerKnowledgeSearch(entry);
}

// viewer.js exposes openDoc on window so cards.js (window.openDoc) can reach it
window.openDoc = openDoc;

// ── openKbDoc ─────────────────────────────────────────────────────────────────

export async function openKbDoc(kbHit) {
  const { repo, path, url, title } = kbHit;

  // Set KB viewer state
  state.viewer.entry = null;
  state.viewer.isKb = true;
  state.viewer.kbRepo = repo;
  state.viewer.kbPath = path;
  state.viewer.layer = 'raw';
  state.viewer.rawText = '';
  state.viewer.annotation = {};
  state.viewer.lang = null;
  exitEditMode(false);
  hideCommitBar();

  // Header setup
  const _kbPathParts = (path || '').split('/');
  const _kbFileName = _kbPathParts.pop();
  const _kbRepoName = (repo || '').split('/').pop();
  document.getElementById('md-panel-title').textContent =
    _kbPathParts.length > 0 ? `${_kbRepoName}/.../${_kbFileName}` : `${_kbRepoName}/${_kbFileName}`;
  document.getElementById('md-github-link').href = url || '#';
  document.getElementById('btn-copy-http').dataset.url = url || '';
  document.getElementById('btn-copy-http').dataset.tip = url || '';
  document.getElementById('btn-copy-path').style.display = 'none';
  document.getElementById('btn-goto-kb').style.display = 'none';
  document.getElementById('md-lang-bar').style.display = 'none';
  document.getElementById('md-file-size').textContent = '';
  document.getElementById('btn-add-comment').style.display = 'none';
  document.getElementById('btn-panel-commit').style.display = 'none';

  // Show iTerm button for KB mode
  const itermBtn = document.getElementById('btn-open-iterm');
  itermBtn.style.display = '';
  itermBtn.onclick = async () => {
    itermBtn.disabled = true;
    try {
      const res = await api.openItermAt(repo);
      if (res.error) alert(`打开终端失败：${res.error}`);
    } catch (e) {
      alert(`打开终端失败：${e.message}`);
    } finally {
      itermBtn.disabled = false;
    }
  };

  // Open modal
  const modal = document.getElementById('md-modal');
  const body = document.getElementById('md-body');
  body.innerHTML = '<div style="color:#8c959f;padding:20px;font-size:13px;">加载中…</div>';
  modal.style.display = 'flex';
  document.body.style.overflow = 'hidden';

  try {
    const result = await api.fetchKbFileContent(repo, path);
    if (result.error) throw new Error(result.error);
    const text = result.content;
    state.viewer.rawText = text;
    const bytes = new Blob([text]).size;
    document.getElementById('md-file-size').textContent = bytes < 1024
      ? `${bytes} B`
      : bytes < 1024 * 1024
        ? `${(bytes / 1024).toFixed(1)} KB`
        : `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    if (typeof marked !== 'undefined') {
      body.innerHTML = marked.parse(text);
    } else {
      body.innerHTML = `<pre style="white-space:pre-wrap;font-size:13px">${escHtml(text)}</pre>`;
    }
    postProcessLinks(body, 'raw', path);
    document.getElementById('knowledge-panel').style.display = 'none';
    document.getElementById('btn-edit').style.display = '';
  } catch (e) {
    body.innerHTML = `<div style="color:#7d4e00;padding:20px">无法加载文件：${escHtml(e.message)}</div>`;
    document.getElementById('btn-edit').style.display = 'none';
  }
}

window.openKbDoc = openKbDoc;

// ── KB save / commit (internal) ───────────────────────────────────────────

async function _saveKbDoc() {
  const editArea = document.getElementById('md-edit-area');
  const newContent = editArea.value;
  const btnSave = document.getElementById('btn-save');
  btnSave.disabled = true;
  btnSave.textContent = '保存中…';
  try {
    const data = await api.saveKbFile(state.viewer.kbRepo, state.viewer.kbPath, newContent);
    if (data.error) throw new Error(data.error);
    state.viewer.rawText = newContent;
    exitEditMode(false);
    // Re-render content
    const body = document.getElementById('md-body');
    if (typeof marked !== 'undefined') {
      body.innerHTML = marked.parse(newContent);
    } else {
      body.innerHTML = `<pre style="white-space:pre-wrap;font-size:13px">${escHtml(newContent)}</pre>`;
    }
    postProcessLinks(body, 'raw', state.viewer.kbPath);
    document.getElementById('knowledge-panel').style.display = 'none';
    document.getElementById('btn-add-comment').style.display = 'none';
    document.getElementById('btn-edit').style.display = '';
    showCommitBar();
    _showKbReindexBtn(state.viewer.kbRepo);
  } catch (e) {
    alert(`保存失败：${e.message}\n\n请确认已通过 python3 server.py 启动服务器。`);
  } finally {
    btnSave.disabled = false;
    btnSave.textContent = '💾 保存';
  }
}

async function _commitCurrentKbFile() {
  const msg = document.getElementById('md-commit-msg').value.trim() || 'update: edit via viewer';
  const resultEl = document.getElementById('md-commit-result');
  const btn = document.getElementById('btn-commit-file');
  btn.disabled = true;
  btn.textContent = '提交中…';
  resultEl.style.color = '#57606a';
  resultEl.textContent = '';

  try {
    const data = await api.commitKbFile(state.viewer.kbRepo, state.viewer.kbPath, msg);
    if (data.error) throw new Error(data.error + (data.stderr ? '\n' + data.stderr : ''));
    resultEl.style.color = '#1a7f37';
    resultEl.textContent = '✓ 已推送！';
    _showKbReindexBtn(state.viewer.kbRepo);
    setTimeout(hideCommitBar, 2000);
  } catch (e) {
    resultEl.style.color = '#cf222e';
    resultEl.textContent = `✗ ${e.message}`;
  } finally {
    btn.disabled = false;
    btn.textContent = '↑ 提交';
  }
}

function _showKbReindexBtn(repo) {
  let btn = document.getElementById('btn-kb-reindex');
  if (!btn) {
    btn = document.createElement('button');
    btn.id = 'btn-kb-reindex';
    btn.className = 'md-header-btn';
    const closeBtn = document.getElementById('md-close');
    closeBtn.parentNode.insertBefore(btn, closeBtn);
  }
  btn.textContent = '↺ 重建索引';
  btn.title = '重建此知识库的搜索索引';
  btn.disabled = false;
  btn.style.display = '';
  btn.onclick = async () => {
    btn.disabled = true;
    btn.textContent = '重建中…';
    try {
      const res = await api.reindexKbRepo(repo);
      if (res.error) throw new Error(res.error);
      const poll = setInterval(async () => {
        try {
          const status = await api.getReindexStatus();
          if (status.status === 'done') {
            clearInterval(poll);
            btn.textContent = '✓ 已重建';
            btn.disabled = false;
            setTimeout(() => { btn.style.display = 'none'; }, 2000);
          } else if (status.status === 'error') {
            clearInterval(poll);
            btn.textContent = '重建失败';
            btn.disabled = false;
            btn.title = status.log || '未知错误';
          }
        } catch (_) {}
      }, 2000);
    } catch (e) {
      btn.textContent = '重建失败';
      btn.disabled = false;
      btn.title = e.message;
    }
  };
}

// ── switchLang ─────────────────────────────────────────────────────────────

export async function switchLang(lang) {
  if (!state.viewer.entry) return;
  const entry = state.viewer.entry;
  const layer = state.viewer.layer;

  // save current scroll position
  const body = document.getElementById('md-body');
  const cacheKey = `${getEntryId(entry)}:${layer}`;
  state.viewer.scrollCache[cacheKey] = body.scrollTop;

  state.viewer.lang = lang;
  const activePath = getActivePath(entry, lang);
  updateHeaderUrls(entry, layer, activePath);
  updateLangBar(entry);

  body.innerHTML = '<div style="color:#8c959f;padding:20px;font-size:13px;">加载中…</div>';

  try {
    const text = await api.fetchFileContent(layer, activePath);
    state.viewer.rawText = text;
    const bytes = new Blob([text]).size;
    document.getElementById('md-file-size').textContent = bytes < 1024
      ? `${bytes} B`
      : bytes < 1024 * 1024
        ? `${(bytes / 1024).toFixed(1)} KB`
        : `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    renderDocBody(text, layer, activePath);
    applyHighlights(state.viewer.annotation, layer);
    // restore scroll position
    const cacheKey = `${getEntryId(entry)}:${layer}`;
    const saved = state.viewer.scrollCache[cacheKey];
    if (saved != null) requestAnimationFrame(() => { body.scrollTop = saved; });
  } catch (e) {
    body.innerHTML = `<div style="color:#7d4e00;padding:20px">无法加载文件：${escHtml(e.message)}</div>`;
  }
}

// ── Edit mode ──────────────────────────────────────────────────────────────

export function enterEditMode() {
  const editArea = document.getElementById('md-edit-area');
  const body = document.getElementById('md-body');
  editArea.value = state.viewer.rawText;
  body.style.display = 'none';
  editArea.style.display = '';
  requestAnimationFrame(() => { editArea.scrollTop = 0; });
  editArea.focus();
  document.getElementById('btn-edit').style.display = 'none';
  document.getElementById('btn-add-comment').style.display = 'none';
  document.getElementById('btn-panel-commit').style.display = 'none';
  document.getElementById('btn-save').style.display = '';
  document.getElementById('btn-cancel-edit').style.display = '';
  document.getElementById('md-github-link').style.display = 'none';
  hideCommitBar();
}

export function exitEditMode(rerender = true) {
  const editArea = document.getElementById('md-edit-area');
  const body = document.getElementById('md-body');
  editArea.style.display = 'none';
  body.style.display = '';
  document.getElementById('btn-edit').style.display = '';
  document.getElementById('btn-add-comment').style.display = '';
  document.getElementById('btn-save').style.display = 'none';
  document.getElementById('btn-cancel-edit').style.display = 'none';
  document.getElementById('btn-panel-commit').style.display = 'none';
  document.getElementById('md-github-link').style.display = '';
  if (rerender && state.viewer.entry) {
    const activePath = getActivePath(state.viewer.entry, state.viewer.lang);
    renderDocBody(state.viewer.rawText, state.viewer.layer, activePath);
  }
}

export async function saveDoc() {
  if (state.viewer.isKb) {
    await _saveKbDoc();
    return;
  }
  if (!state.viewer.entry) return;
  const editArea = document.getElementById('md-edit-area');
  const newContent = editArea.value;
  const btnSave = document.getElementById('btn-save');
  btnSave.disabled = true;
  btnSave.textContent = '保存中…';

  try {
    const activePath = getActivePath(state.viewer.entry, state.viewer.lang);
    const data = await api.saveFile(state.viewer.layer, activePath, newContent);
    if (data.error) throw new Error(data.error);

    state.viewer.rawText = newContent;
    const date = state.viewer.entry.created_at.slice(0, 8);
    const h1Match = newContent.match(/^#\s+(.+)/m);
    const newTitle = h1Match ? h1Match[1].trim() : slugToTitle(filenameFromPath(state.viewer.entry.common_path));
    const entryId = getEntryId(state.viewer.entry);
    if (!state.index.titleCache.has(date)) state.index.titleCache.set(date, new Map());
    state.index.titleCache.get(date).set(entryId, newTitle);
    const card = entryId ? document.querySelector(`.doc-card[data-id="${entryId}"]`) : null;
    if (card) {
      const titleEl = card.querySelector('.doc-title-btn');
      if (titleEl) { titleEl.classList.remove('loading'); titleEl.textContent = newTitle; }
    } else {
      updateTitlesInDOM(date);
    }
    exitEditMode(true);
    await loadDiffStatus();
    updateDiffInDOM();
    showCommitBar();
  } catch (e) {
    alert(`保存失败：${e.message}\n\n请确认已通过 python3 server.py 启动服务器。`);
  } finally {
    btnSave.disabled = false;
    btnSave.textContent = '💾 保存';
  }
}

// bridge: exitEditMode still called by openDoc in this file (no window needed internally)
// index.html Step 4.2 removes window.exitEditMode assignment

// ── Commit bar ─────────────────────────────────────────────────────────────

export function showCommitBar() {
  document.getElementById('md-commit-msg').value = '';
  document.getElementById('md-commit-result').textContent = '';
  document.getElementById('btn-panel-commit').style.display = 'none';
  document.getElementById('md-commit-bar').style.display = 'flex';
  document.getElementById('md-commit-msg').focus();
}

export function hideCommitBar() {
  document.getElementById('md-commit-bar').style.display = 'none';
}

export async function commitCurrentFile() {
  if (state.viewer.isKb) {
    await _commitCurrentKbFile();
    return;
  }
  if (!state.viewer.entry) return;
  const msg = document.getElementById('md-commit-msg').value.trim() || 'update: edit via viewer';
  const resultEl = document.getElementById('md-commit-result');
  const btn = document.getElementById('btn-commit-file');
  btn.disabled = true;
  btn.textContent = '提交中…';
  resultEl.style.color = '#57606a';
  resultEl.textContent = '';

  try {
    const activePath = getActivePath(state.viewer.entry, state.viewer.lang);
    const filePath = `${state.viewer.layer}/${activePath}`;
    const data = await api.commitFiles(msg, [filePath]);
    if (data.error) throw new Error(data.error + (data.stderr ? '\n' + data.stderr : ''));

    resultEl.style.color = '#1a7f37';
    resultEl.textContent = '✓ 已推送！';
    await loadDiffStatus();
    updateDiffInDOM();
    document.dispatchEvent(new CustomEvent('cta:reload'));
    setTimeout(hideCommitBar, 1500);
  } catch (e) {
    resultEl.style.color = '#cf222e';
    resultEl.textContent = `✗ ${e.message}`;
  } finally {
    btn.disabled = false;
    btn.textContent = '↑ 提交';
  }
}

// ── Close viewer ───────────────────────────────────────────────────────────

export function closeModal() {
  document.getElementById('md-modal').style.display = 'none';
  document.body.style.overflow = '';
  exitEditMode(false);
  hideCommitBar();
  // Reset KB state and restore hidden elements
  if (state.viewer.isKb) {
    state.viewer.isKb = false;
    state.viewer.kbRepo = null;
    state.viewer.kbPath = null;
    document.getElementById('btn-copy-path').style.display = '';
    document.getElementById('btn-add-comment').style.display = '';
    document.getElementById('knowledge-panel').style.display = '';
    document.getElementById('btn-open-iterm').style.display = 'none';
    const kbReindexBtn = document.getElementById('btn-kb-reindex');
    if (kbReindexBtn) kbReindexBtn.style.display = 'none';
  }
}

document.getElementById('md-close').addEventListener('click', closeModal);
document.getElementById('md-backdrop').addEventListener('click', closeModal);
document.addEventListener('keydown', e => { if (e.key === 'Escape') closeModal(); });

// ── Copy buttons ───────────────────────────────────────────────────────────

document.getElementById('btn-copy-http').addEventListener('click', () => {
  const url = document.getElementById('btn-copy-http').dataset.url || '';
  if (!url) return;
  navigator.clipboard.writeText(url).then(() => {
    const btn = document.getElementById('btn-copy-http');
    btn.textContent = '✓';
    setTimeout(() => { btn.textContent = '🌐'; }, 1200);
  });
});

document.getElementById('btn-copy-path').addEventListener('click', () => {
  const activePath = getActivePath(state.viewer.entry, state.viewer.lang);
  const relPath = `${state.viewer.layer}/${activePath}`;
  const fullPath = state.ui.archiveRoot ? `${state.ui.archiveRoot}/${relPath}` : relPath;
  navigator.clipboard.writeText(fullPath).then(() => {
    const btn = document.getElementById('btn-copy-path');
    btn.textContent = '✓';
    setTimeout(() => { btn.textContent = '📂'; }, 1200);
  });
});

document.getElementById('btn-lang-en').addEventListener('click', () => switchLang('en'));
document.getElementById('btn-lang-zh').addEventListener('click', () => switchLang('zh'));

initHighlightUI();

