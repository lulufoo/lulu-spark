import { applyCachedHighlights } from '../../doc-editor/highlights.ts';
import { notesDocKey } from '../../doc-editor/identity.ts';
import { renderDocMarkdown, setDocEditMode } from '../../doc-editor/view.tsx';
import * as api from '../../host/api.ts';
import { getGithubUserUrl, workbenchGithubBlobBase } from '../../host/constants.ts';
import { state } from '../../host/state.ts';
import { renderMermaidBlocks } from '../../shared/mermaid-render.ts';
import { rememberFilePopupAssetUrl, revokeFilePopupAssetUrls } from '../state/assets.ts';
import { commonPathFromAbsPath } from '../state/store.ts';

function asTextArea(el: HTMLElement | null) {
  if (!el || !('value' in el)) return null;
  return el as HTMLTextAreaElement;
}

function resolveRelativeLink(href: string, layer: string, commonPath: string) {
  const ghBase = workbenchGithubBlobBase(getGithubUserUrl(), state.ui.workbenchRoot);
  if (!ghBase) return null;
  try {
    const base = `http://x/notes/${layer}/${commonPath}`;
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
    if (!commonPath) return;
    const ghUrl = resolveRelativeLink(href, layer, commonPath);
    if (ghUrl) {
      (a as HTMLAnchorElement).href = ghUrl;
      (a as HTMLAnchorElement).target = '_blank';
      a.setAttribute('rel', 'noopener noreferrer');
    }
  });
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
        const blobUrl = (await api.fetchNotesAssetAsBlobUrl(layer, commonPath, href)) as string;
        rememberFilePopupAssetUrl(blobUrl);
        (img as HTMLImageElement).src = blobUrl;
      } catch {
        if (!(img as HTMLImageElement).alt) (img as HTMLImageElement).alt = href;
      }
    }),
  );
}

export async function paintFilePopupDoc({
  editing,
  content,
  path,
  layer,
}: {
  editing: boolean;
  content: string;
  path: string;
  layer: 'raw' | 'digest';
}) {
  const bodyEl = document.getElementById('file-popup-body');
  const editAreaEl = asTextArea(document.getElementById('file-popup-edit-area'));
  setDocEditMode({ bodyEl, editAreaEl, text: content, editing });
  if (editing || !bodyEl) return;
  const commonPath = commonPathFromAbsPath(path);
  renderDocMarkdown(bodyEl, content);
  postProcessLinks(bodyEl, layer, commonPath);
  revokeFilePopupAssetUrls();
  if (commonPath) await postProcessImages(bodyEl, layer, commonPath);
  await renderMermaidBlocks(bodyEl);
  if (!commonPath) return;
  void applyCachedHighlights({
    bodyEl,
    identityKey: notesDocKey(commonPath),
  });
}
