/** Shared view / edit surface. Does not own comments or save protocols. */

import { renderToHtml } from '../island.ts';

export function renderDocMarkdown(bodyEl: HTMLElement | null | undefined, text: unknown) {
  if (!bodyEl) return;
  const raw = text == null ? '' : String(text);
  if (typeof marked !== 'undefined') {
    bodyEl.innerHTML = marked.parse(raw);
    return;
  }
  bodyEl.innerHTML = renderToHtml(
    <pre style={{ whiteSpace: 'pre-wrap', fontSize: 13 }}>{raw}</pre>,
  );
}

export function setDocEditMode({
  bodyEl,
  editAreaEl,
  text,
  editing,
}: {
  bodyEl?: HTMLElement | null;
  editAreaEl?: HTMLTextAreaElement | HTMLInputElement | null;
  text?: unknown;
  editing?: boolean;
}) {
  if (!bodyEl || !editAreaEl) return;
  if (editing) {
    editAreaEl.value = text == null ? '' : String(text);
    bodyEl.style.display = 'none';
    editAreaEl.style.display = '';
    return;
  }
  editAreaEl.style.display = 'none';
  bodyEl.style.display = '';
}
