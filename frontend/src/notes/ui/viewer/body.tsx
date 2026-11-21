import { getEntryId, state } from '../../state/host.ts';
import * as api from '../../../host/api.ts';
import { docKey } from '../../../doc-editor/identity.ts';
import { applyCachedHighlights, initDocHighlightOverlay } from '../../../doc-editor/highlights.ts';
import type { OpenHighlightComment } from '../../../doc-editor/types.ts';
import { mountBoardSketches, releaseBoardSketches } from '../../../board/commands/sketch-links.ts';
import { renderDocMarkdown } from '../../../doc-editor/view.tsx';
import { renderMermaidBlocks } from '../../../shared/mermaid-render.ts';
import { closeHighlightCommentEditor, openHighlightCommentEditor } from './highlight-comment.ts';

function postProcessLinks(container: Element) {
  container.querySelectorAll('a[href]').forEach((a) => {
    const href = a.getAttribute('href');
    if (!href || href.startsWith('#')) return;
    if (href.startsWith('http://') || href.startsWith('https://')) {
      (a as HTMLAnchorElement).target = '_blank';
      a.setAttribute('rel', 'noopener noreferrer');
    }
  });
}

const _notesBlobUrls = new Set<string>();

function revokeNotesBlobUrls() {
  for (const url of _notesBlobUrls) {
    URL.revokeObjectURL(url);
  }
  _notesBlobUrls.clear();
}

function isExternalOrSpecialImgSrc(src: string) {
  return /^(https?:|data:|blob:|\/)/i.test(src);
}

async function postProcessImages(container: Element, layer: string, id: string) {
  const imgs = [...container.querySelectorAll('img[src]')];
  await Promise.all(
    imgs.map(async (img) => {
      const href = img.getAttribute('src');
      if (!href || href.startsWith('#') || isExternalOrSpecialImgSrc(href)) return;
      try {
        const blobUrl = (await api.fetchNotesAssetAsBlobUrl(layer, id, href)) as string;
        _notesBlobUrls.add(blobUrl);
        (img as HTMLImageElement).src = blobUrl;
      } catch {
        if (!(img as HTMLImageElement).alt) (img as HTMLImageElement).alt = href;
      }
    }),
  );
}

function notesIdentityKey() {
  const id = getEntryId(state.viewer.entry);
  return id ? docKey(id) : '';
}

const openNotesHighlightComment: OpenHighlightComment = (identityKey, highlight, anchor, point) => {
  if (notesIdentityKey() !== identityKey || state.viewer.editing || state.viewer.loading) return;
  openHighlightCommentEditor({
    anchor,
    point,
    text: highlight.comments?.[0]?.text ?? '',
    async onSave(text) {
      const saved = await api.setDocHighlightComment(identityKey, highlight.id, text);
      highlight.comments = saved.comments;
    },
  });
};

export function initHighlightUI() {
  initDocHighlightOverlay({
    getBody: () => document.getElementById('md-body'),
    getEditArea: () => document.getElementById('md-edit-area'),
    getIdentityKey: () => notesIdentityKey(),
    excludeBarId: 'md-comments-bar',
    buttonId: 'highlight-add-btn',
    onOpenComment: openNotesHighlightComment,
  });
}

export async function renderDocBody(text: string, layer: string, id: string) {
  const body = document.getElementById('md-body');
  if (!body) return;
  closeHighlightCommentEditor();
  const paintKey = state.viewer.bodyPaintKey;
  const identityKey = id ? docKey(id) : '';
  const isCurrent = () => body.isConnected
    && state.viewer.bodyPaintKey === paintKey
    && notesIdentityKey() === identityKey
    && !state.viewer.editing
    && !state.viewer.loading;
  releaseBoardSketches(body);
  renderDocMarkdown(body, text);
  postProcessLinks(body);
  void mountBoardSketches(body);
  revokeNotesBlobUrls();
  await postProcessImages(body, layer, id);
  await renderMermaidBlocks(body);
  void applyCachedHighlights({
    bodyEl: body,
    identityKey,
    excludeBarId: 'md-comments-bar',
    onOpenComment: openNotesHighlightComment,
    isCurrent,
  });
}
