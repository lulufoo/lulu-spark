import { applyCachedHighlights } from '../../doc-editor/highlights.ts';
import { renderDocMarkdown, setDocEditMode } from '../../doc-editor/view.tsx';
import { renderMermaidBlocks } from '../../shared/mermaid-render.ts';

function asTextArea(el: HTMLElement | null) {
  if (!el || !('value' in el)) return null;
  return el as HTMLTextAreaElement;
}

function markExternalLinks(container: Element) {
  container.querySelectorAll('a[href]').forEach((anchor) => {
    const href = anchor.getAttribute('href');
    if (!href || href.startsWith('#')) return;
    if (href.startsWith('http://') || href.startsWith('https://')) {
      (anchor as HTMLAnchorElement).target = '_blank';
      anchor.setAttribute('rel', 'noopener noreferrer');
    }
  });
}

export async function paintFilePopupDoc({
  editing,
  content,
  identityKey,
}: {
  editing: boolean;
  content: string;
  identityKey?: string;
}) {
  const bodyEl = document.getElementById('file-popup-body');
  const editAreaEl = asTextArea(document.getElementById('file-popup-edit-area'));
  setDocEditMode({ bodyEl, editAreaEl, text: content, editing });
  if (editing || !bodyEl) return;
  renderDocMarkdown(bodyEl, content);
  markExternalLinks(bodyEl);
  await renderMermaidBlocks(bodyEl);
  const key = String(identityKey || '').trim();
  if (!key) return;
  await applyCachedHighlights({
    bodyEl,
    identityKey: key,
  });
}
