/** Plain-text model of the contenteditable composer. Pure DOM helpers; no React. */

const BLOCK_TAGS = new Set(['DIV', 'P']);

/**
 * The composer's text. Text nodes read as-is; `<br>` and block elements read as newlines,
 * except a `<br>` that ends its parent (the browser's placeholder for an empty line).
 */
export function readComposerText(root: Node): string {
  let out = '';
  const walk = (node: Node) => {
    for (const child of Array.from(node.childNodes)) {
      if (child.nodeType === Node.TEXT_NODE) {
        out += child.nodeValue ?? '';
        continue;
      }
      if (!(child instanceof HTMLElement)) continue;
      if (child.tagName === 'BR') {
        if (child.nextSibling) out += '\n';
        continue;
      }
      if (BLOCK_TAGS.has(child.tagName) && out !== '' && !out.endsWith('\n')) out += '\n';
      walk(child);
    }
  };
  walk(root);
  return out;
}

export function composerIsEmpty(root: Node): boolean {
  return readComposerText(root) === '';
}

/** Keep `data-empty` (placeholder visibility) in step with the content; drop stray `<br>`. */
export function syncEmptyMarker(root: HTMLElement): void {
  const empty = composerIsEmpty(root);
  if (empty && root.childNodes.length > 0) root.textContent = '';
  root.setAttribute('data-empty', String(empty));
}

function placeCaretAtEnd(root: HTMLElement): void {
  const selection = root.ownerDocument.getSelection();
  if (!selection) return;
  const range = root.ownerDocument.createRange();
  range.selectNodeContents(root);
  range.collapse(false);
  selection.removeAllRanges();
  selection.addRange(range);
}

/** Replace the content with `text`; the caret goes to the end. */
export function writeComposerText(root: HTMLElement, text: string): void {
  root.textContent = '';
  if (text) root.appendChild(root.ownerDocument.createTextNode(text));
  syncEmptyMarker(root);
  placeCaretAtEnd(root);
}

/** Empty the content without touching the selection. */
export function clearComposerText(root: HTMLElement): void {
  root.textContent = '';
  syncEmptyMarker(root);
}

/** Insert plain text at the caret (replacing any selection). Falls back to the end. */
export function insertPlainTextAtSelection(root: HTMLElement, text: string): void {
  if (!text) return;
  const doc = root.ownerDocument;
  const selection = doc.getSelection();
  if (!selection) return;
  if (selection.rangeCount === 0 || !root.contains(selection.getRangeAt(0).commonAncestorContainer)) {
    placeCaretAtEnd(root);
  }
  const range = selection.getRangeAt(0);
  range.deleteContents();
  const start = range.startContainer;
  if (start.nodeType === Node.TEXT_NODE) {
    const textNode = start as Text;
    const offset = range.startOffset;
    textNode.insertData(offset, text);
    range.setStart(textNode, offset + text.length);
  } else {
    const node = doc.createTextNode(text);
    range.insertNode(node);
    range.setStart(node, node.length);
  }
  range.collapse(true);
  selection.removeAllRanges();
  selection.addRange(range);
}

function notifyInput(root: HTMLElement): void {
  root.dispatchEvent(new Event('input', { bubbles: true }));
}

/**
 * Keep the editable plain: Enter / Shift+Enter become a `\n` character, paste and drop keep
 * text only, rich-text commands are refused. Returns the detach function.
 */
export function attachComposerEditing(root: HTMLElement): () => void {
  const onBeforeInput = (event: Event) => {
    const input = event as InputEvent;
    if (input.isComposing) return;
    const type = input.inputType ?? '';
    if (type === 'insertLineBreak' || type === 'insertParagraph') {
      event.preventDefault();
      insertPlainTextAtSelection(root, '\n');
      notifyInput(root);
    } else if (type.startsWith('format')) {
      event.preventDefault();
    }
  };
  const insertFromTransfer = (event: Event, data: DataTransfer | null | undefined) => {
    event.preventDefault();
    const text = (data?.getData('text/plain') ?? '').replace(/\r\n?/g, '\n');
    if (!text) return;
    insertPlainTextAtSelection(root, text);
    notifyInput(root);
  };
  const onPaste = (event: Event) => insertFromTransfer(event, (event as ClipboardEvent).clipboardData);
  const onDrop = (event: Event) => insertFromTransfer(event, (event as DragEvent).dataTransfer);
  const onInput = () => syncEmptyMarker(root);

  root.addEventListener('beforeinput', onBeforeInput);
  root.addEventListener('paste', onPaste);
  root.addEventListener('drop', onDrop);
  root.addEventListener('input', onInput);
  syncEmptyMarker(root);
  return () => {
    root.removeEventListener('beforeinput', onBeforeInput);
    root.removeEventListener('paste', onPaste);
    root.removeEventListener('drop', onDrop);
    root.removeEventListener('input', onInput);
  };
}
