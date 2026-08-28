import { escHtml } from '../../shared/utils.ts';
import { renderMermaidBlocks } from '../../shared/mermaid-render.ts';

/** Read-only Home chat markdown. Uses marked + mermaid; not the Notes editor. */

export function renderHomeChatMarkdown(text: unknown): string {
  const source = text == null ? '' : String(text);
  if (typeof marked !== 'undefined' && typeof marked.parse === 'function') {
    return marked.parse(source);
  }
  return `<pre>${escHtml(source)}</pre>`;
}

export async function hydrateHomeChatMarkdown(container: Element | null): Promise<void> {
  if (!container) return;
  await renderMermaidBlocks(container);
}
