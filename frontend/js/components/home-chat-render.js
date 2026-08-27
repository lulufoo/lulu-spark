import { escHtml } from '../utils.js';
import { renderMermaidBlocks } from '../mermaid-render.js';

/** Read-only Home chat markdown. Uses marked + mermaid; not the Notes editor. */

export function renderHomeChatMarkdown(text) {
  const source = text == null ? '' : String(text);
  if (typeof marked !== 'undefined' && typeof marked.parse === 'function') {
    return marked.parse(source);
  }
  return `<pre>${escHtml(source)}</pre>`;
}

export async function hydrateHomeChatMarkdown(container) {
  if (!container) return;
  await renderMermaidBlocks(container);
}
