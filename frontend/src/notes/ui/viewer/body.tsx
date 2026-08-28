// @ts-nocheck — ported from JS; state shapes stay unchecked like checkJs:false.
import { state } from '../../state/host.ts';
import { getGithubUserUrl, workbenchGithubBlobBase } from '../../../host/constants.ts';
import * as api from '../../../host/api.ts';
import { notesDocKey } from '../../../doc-editor/identity.ts';
import { applyCachedHighlights, initDocHighlightOverlay } from '../../../doc-editor/highlights.ts';
import { renderDocMarkdown } from '../../../doc-editor/view.tsx';
import { renderMermaidBlocks } from '../../../shared/mermaid-render.ts';

function resolveRelativeLink(href: string, layer: string, commonPath: string) {
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

function postProcessLinks(container: Element, layer: string, commonPath: string) {
  container.querySelectorAll('a[href]').forEach((a) => {
    const href = a.getAttribute('href');
    if (!href || href.startsWith('#')) return;
    if (href.startsWith('http://') || href.startsWith('https://')) {
      (a as HTMLAnchorElement).target = '_blank';
      a.setAttribute('rel', 'noopener noreferrer');
      return;
    }
    const ghUrl = resolveRelativeLink(href, layer, commonPath);
    if (ghUrl) {
      (a as HTMLAnchorElement).href = ghUrl;
      (a as HTMLAnchorElement).target = '_blank';
      a.setAttribute('rel', 'noopener noreferrer');
    }
  });
}

const _corpusBlobUrls = new Set<string>();

function revokeCorpusBlobUrls() {
  for (const url of _corpusBlobUrls) {
    URL.revokeObjectURL(url);
  }
  _corpusBlobUrls.clear();
}

function isExternalOrSpecialImgSrc(src: string) {
  return /^(https?:|data:|blob:|\/)/i.test(src);
}

async function postProcessImages(container: Element, layer: string, commonPath: string) {
  const imgs = [...container.querySelectorAll('img[src]')];
  await Promise.all(
    imgs.map(async (img) => {
      const href = img.getAttribute('src');
      if (!href || href.startsWith('#') || isExternalOrSpecialImgSrc(href)) return;
      try {
        const blobUrl = await api.fetchCorpusAssetAsBlobUrl(layer, commonPath, href);
        _corpusBlobUrls.add(blobUrl);
        (img as HTMLImageElement).src = blobUrl;
      } catch {
        if (!(img as HTMLImageElement).alt) (img as HTMLImageElement).alt = href;
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

export async function renderDocBody(text: string, layer: string, commonPath: string) {
  const body = document.getElementById('md-body');
  if (!body) return;
  renderDocMarkdown(body, text);
  postProcessLinks(body, layer, commonPath);
  revokeCorpusBlobUrls();
  await postProcessImages(body, layer, commonPath);
  await renderMermaidBlocks(body);
  void applyHighlights();
}
