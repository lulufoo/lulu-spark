/** Shared view / edit surface. Does not own comments or save protocols. */

export function renderDocMarkdown(bodyEl, text) {
  if (!bodyEl) return;
  const raw = text == null ? '' : String(text);
  if (typeof marked !== 'undefined') {
    bodyEl.innerHTML = marked.parse(raw);
    return;
  }
  const esc = raw
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
  bodyEl.innerHTML = `<pre style="white-space:pre-wrap;font-size:13px">${esc}</pre>`;
}

export function setDocEditMode({ bodyEl, editAreaEl, text, editing }) {
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
