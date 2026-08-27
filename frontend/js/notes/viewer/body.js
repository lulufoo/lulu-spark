import { state } from '../../host/state.js';
import { getGithubUserUrl, workbenchGithubBlobBase } from '../../host/constants.js';
import { escHtml } from '../../shared/utils.js';
import * as api from '../../host/api.js';
import { renderLinksBar } from '../links-bar.js';
import { renderTagsBar } from '../tags-bar.js';
import { renderComments } from '../comments.js';
import { openDeleteDialog } from '../delete-dialog.js';
import { notesDocKey } from '../../doc-editor/identity.js';
import { applyCachedHighlights, initDocHighlightOverlay } from '../../doc-editor/highlights.js';
import { renderDocMarkdown } from '../../doc-editor/view.js';
import { renderMermaidBlocks } from '../../shared/mermaid-render.js';

function resolveRelativeLink(href, layer, commonPath) {
  const ghBase = workbenchGithubBlobBase(getGithubUserUrl(), state.ui.workbenchKnowledgeRoot);
  if (!ghBase) return null;
  try {
    const base = `http://x/${layer}/${commonPath}`;
    const resolved = new URL(href, base);
    const repoPath = resolved.pathname.slice(1);
    return `${ghBase}/${repoPath}`;
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

const _corpusBlobUrls = new Set();

function revokeCorpusBlobUrls() {
  for (const url of _corpusBlobUrls) {
    URL.revokeObjectURL(url);
  }
  _corpusBlobUrls.clear();
}

function isExternalOrSpecialImgSrc(src) {
  return /^(https?:|data:|blob:|\/)/i.test(src);
}

async function postProcessImages(container, layer, commonPath) {
  const imgs = [...container.querySelectorAll('img[src]')];
  await Promise.all(
    imgs.map(async (img) => {
      const href = img.getAttribute('src');
      if (!href || href.startsWith('#') || isExternalOrSpecialImgSrc(href)) return;
      try {
        const blobUrl = await api.fetchCorpusAssetAsBlobUrl(layer, commonPath, href);
        _corpusBlobUrls.add(blobUrl);
        img.src = blobUrl;
      } catch {
        img.alt = img.alt || href;
      }
    }),
  );
}

function notesIdentityKey() {
  const path = state.viewer.entry?.common_path;
  return path ? notesDocKey(path) : '';
}

function applyHighlights() {
  const body = document.getElementById('md-body');
  const identityKey = notesIdentityKey();
  if (!body || !identityKey) return;
  void applyCachedHighlights({
    bodyEl: body,
    identityKey,
    excludeBarId: 'md-comments-bar',
  });
}

export function initHighlightUI() {
  initDocHighlightOverlay({
    getBody: () => document.getElementById('md-body'),
    getEditArea: () => document.getElementById('md-edit-area'),
    getIdentityKey: () => notesIdentityKey(),
    excludeBarId: 'md-comments-bar',
    buttonId: 'highlight-add-btn',
  });
}

// ── renderDocBody ──────────────────────────────────────────────────────────

export async function renderDocBody(text, layer, commonPath) {
  const body = document.getElementById('md-body');
  renderDocMarkdown(body, text);
  postProcessLinks(body, layer, commonPath);
  document.getElementById('btn-edit').style.display = '';
  renderLinksBar(state.viewer.entry);
  renderTagsBar(state.viewer.entry);
  revokeCorpusBlobUrls();
  await postProcessImages(body, layer, commonPath);
  await renderMermaidBlocks(body);
  renderComments(state.viewer.annotation, layer, state.viewer.entry);
  const zone = document.createElement('div');
  zone.className = 'md-body-delete-zone';
  const delBtn = document.createElement('button');
  delBtn.id = 'btn-delete';
  delBtn.textContent = '🗑 Delete this entry';
  delBtn.title = 'Deletes all linked files (raw / distilled / trace / digest / diagnose)';
  delBtn.addEventListener('click', () => openDeleteDialog());
  zone.appendChild(delBtn);
  body.appendChild(zone);
  void applyHighlights();
}
