/** Text model of the contenteditable composer. Pure DOM helpers; no React.
 * Reference chips are visual only: the text it reads and writes keeps the `[title](scheme:id)` links. */
import {
  CHIP_CLASS,
  chipFromNode,
  chipValue,
  createChipElement,
  parseReferences,
  referenceKindByName,
  serializeChip,
  serializeReference,
  type ReferenceValue,
} from '../references/index.ts';
import { deleteComposerLineBackward } from './composer-line-delete.ts';

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
      if (child.classList.contains(CHIP_CLASS)) {
        out += serializeChip(child);
        continue;
      }
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

/** Text runs become text nodes, valid references become chips. */
function buildFragment(doc: Document, text: string): DocumentFragment {
  const fragment = doc.createDocumentFragment();
  for (const segment of parseReferences(text)) {
    fragment.appendChild(
      segment.type === 'text'
        ? doc.createTextNode(segment.text)
        : createChipElement(doc, segment),
    );
  }
  return fragment;
}

/** Replace the content with `text`; the caret goes to the end. */
export function writeComposerText(root: HTMLElement, text: string): void {
  root.textContent = '';
  root.appendChild(buildFragment(root.ownerDocument, text));
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
  const fragment = buildFragment(doc, text);
  const hasChip = fragment.querySelector(`.${CHIP_CLASS}`) !== null;
  const start = range.startContainer;
  if (hasChip) {
    const last = fragment.lastChild as Node;
    range.insertNode(fragment);
    if (last.nodeType === Node.TEXT_NODE) range.setStart(last, (last as Text).length);
    else range.setStartAfter(last);
  } else if (start.nodeType === Node.TEXT_NODE) {
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

/** Insert a reference chip (plus a space to keep typing after it) at the caret. */
export function insertReferenceAtSelection(root: HTMLElement, ref: ReferenceValue): void {
  insertPlainTextAtSelection(root, `${serializeReference(ref)} `);
  notifyInput(root);
}

/** The chip a Backspace / Delete at the caret would touch, if any. Only for a collapsed caret. */
function adjacentChip(root: HTMLElement, direction: 'back' | 'forward'): HTMLElement | null {
  const selection = root.ownerDocument.getSelection();
  if (!selection || selection.rangeCount === 0 || !selection.isCollapsed) return null;
  const range = selection.getRangeAt(0);
  const node = range.startContainer;
  if (!root.contains(node)) return null;
  const offset = range.startOffset;
  let neighbor: Node | null;
  if (node.nodeType === Node.TEXT_NODE) {
    const length = (node as Text).length;
    if (direction === 'back') neighbor = offset === 0 ? node.previousSibling : null;
    else neighbor = offset === length ? node.nextSibling : null;
  } else {
    neighbor = (direction === 'back' ? node.childNodes[offset - 1] : node.childNodes[offset]) ?? null;
  }
  return neighbor instanceof HTMLElement && neighbor.classList.contains(CHIP_CLASS) ? neighbor : null;
}

function removeChip(root: HTMLElement, chip: HTMLElement): void {
  const parent = chip.parentNode as Node;
  const index = Array.prototype.indexOf.call(parent.childNodes, chip) as number;
  chip.remove();
  const selection = root.ownerDocument.getSelection();
  if (selection) {
    const range = root.ownerDocument.createRange();
    range.setStart(parent, index);
    range.collapse(true);
    selection.removeAllRanges();
    selection.addRange(range);
  }
  syncEmptyMarker(root);
  notifyInput(root);
}

/** The current non-empty selection inside the composer, as markdown. */
function selectedMarkdown(root: HTMLElement): { range: Range; text: string } | null {
  const selection = root.ownerDocument.getSelection();
  if (!selection || selection.rangeCount === 0 || selection.isCollapsed) return null;
  const range = selection.getRangeAt(0);
  if (!root.contains(range.commonAncestorContainer)) return null;
  return { range, text: readComposerText(range.cloneContents()) };
}

/**
 * Keep the editable plain: Enter / Shift+Enter become a `\n` character, paste and drop keep
 * text only, rich-text commands are refused. Reference chips delete as one unit, open on click
 * and copy as markdown. Returns the detach function.
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
    } else if (type === 'deleteSoftLineBackward' || type === 'deleteHardLineBackward') {
      event.preventDefault();
      deleteComposerLineBackward(root);
    } else if (type === 'deleteContentBackward' || type === 'deleteContentForward') {
      const chip = adjacentChip(root, type === 'deleteContentBackward' ? 'back' : 'forward');
      if (chip) {
        event.preventDefault();
        removeChip(root, chip);
      }
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
  const onCopy = (event: Event) => {
    const selected = selectedMarkdown(root);
    const data = (event as ClipboardEvent).clipboardData;
    if (!selected || !data) return;
    event.preventDefault();
    data.setData('text/plain', selected.text);
  };
  const onCut = (event: Event) => {
    const selected = selectedMarkdown(root);
    const data = (event as ClipboardEvent).clipboardData;
    if (!selected || !data) return;
    event.preventDefault();
    data.setData('text/plain', selected.text);
    selected.range.deleteContents();
    syncEmptyMarker(root);
    notifyInput(root);
  };
  const onClick = (event: Event) => {
    const chip = chipFromNode(event.target as Node | null);
    if (!chip || !root.contains(chip)) return;
    event.preventDefault();
    const value = chipValue(chip);
    const kind = value ? referenceKindByName(value.kind) : undefined;
    if (value && kind) void kind.open(value.id);
  };
  const onInput = () => syncEmptyMarker(root);

  root.addEventListener('beforeinput', onBeforeInput);
  root.addEventListener('paste', onPaste);
  root.addEventListener('drop', onDrop);
  root.addEventListener('copy', onCopy);
  root.addEventListener('cut', onCut);
  root.addEventListener('click', onClick);
  root.addEventListener('input', onInput);
  syncEmptyMarker(root);
  return () => {
    root.removeEventListener('beforeinput', onBeforeInput);
    root.removeEventListener('paste', onPaste);
    root.removeEventListener('drop', onDrop);
    root.removeEventListener('copy', onCopy);
    root.removeEventListener('cut', onCut);
    root.removeEventListener('click', onClick);
    root.removeEventListener('input', onInput);
  };
}
