import { state } from '../state.js'
import { REPO } from '../constants.js'
import { escHtml, filenameFromPath } from '../utils.js'
import * as api from '../api.js'

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
  window.renderLinksBar(state.viewer.entry);
  window.renderComments(state.viewer.annotation, layer, state.viewer.entry);
  const zone = document.createElement('div');
  zone.className = 'md-body-delete-zone';
  const delBtn = document.createElement('button');
  delBtn.id = 'btn-delete';
  delBtn.textContent = '🗑 删除此条目';
  delBtn.title = '删除此条目的所有关联文件（raw / distilled / trace / digest / diagnose）';
  delBtn.addEventListener('click', () => window.openDeleteDialog());
  zone.appendChild(delBtn);
  body.appendChild(zone);
}

// ── openDoc ────────────────────────────────────────────────────────────────

export async function openDoc(entry, layer = 'raw') {
  state.viewer.entry = entry;
  state.viewer.layer = layer;
  state.viewer.annotation = {};
  window.exitEditMode(false);
  window.hideCommitBar();

  const modal = document.getElementById('md-modal');
  const body = document.getElementById('md-body');
  document.getElementById('md-panel-title').textContent = filenameFromPath(entry.common_path).replace(/\.md$/, '');
  const githubUrl = `${REPO}/${layer}/${entry.common_path}`;
  document.getElementById('md-github-link').href = githubUrl;
  document.getElementById('btn-copy-http').dataset.url = githubUrl;
  document.getElementById('btn-copy-http').dataset.tip = githubUrl;
  const relPath = `${layer}/${entry.common_path}`;
  const fullPath = state.ui.archiveRoot ? `${state.ui.archiveRoot}/${relPath}` : relPath;
  document.getElementById('btn-copy-path').dataset.tip = fullPath;
  document.getElementById('md-file-size').textContent = '';
  body.innerHTML = '<div style="color:#8c959f;padding:20px;font-size:13px;">加载中…</div>';
  modal.style.display = 'flex';
  document.body.style.overflow = 'hidden';

  const [mdResult, annResult] = await Promise.allSettled([
    api.fetchFileContent(layer, entry.common_path),
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
  renderDocBody(text, layer, entry.common_path);
  const hasDiff = state.index.diffStatus.get(`${layer}/${entry.common_path}`);
  document.getElementById('btn-panel-commit').style.display = hasDiff ? '' : 'none';
}

// viewer.js exposes openDoc on window so cards.js (window.openDoc) can reach it
window.openDoc = openDoc;
