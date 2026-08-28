import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';
import type { ReactNode } from 'react';

/** Paint a node off-document and return the HTML. Used while controllers still assign innerHTML. */
export function renderToHtml(node: ReactNode): string {
  const box = document.createElement('div');
  const root = createRoot(box);
  flushSync(() => {
    root.render(node);
  });
  const html = box.innerHTML;
  flushSync(() => {
    root.unmount();
  });
  return html;
}

export function mountIsland(container: Element, node: ReactNode): () => void {
  const root: Root = createRoot(container);
  flushSync(() => {
    root.render(node);
  });
  return () => {
    flushSync(() => {
      root.unmount();
    });
    if (container instanceof HTMLElement) container.innerHTML = '';
  };
}
