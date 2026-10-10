/** ⌘⌫ line-delete for the contenteditable composer. Chips count as text on the line. */
import { CHIP_CLASS, serializeChip } from '../references/index.ts';
import { readComposerText, writeComposerText } from './composer-text.ts';

function markdownOffset(root: HTMLElement, container: Node, offset: number): number {
  const prefix = root.ownerDocument.createRange();
  prefix.selectNodeContents(root);
  prefix.setEnd(container, offset);
  return readComposerText(prefix.cloneContents()).length;
}

function placeCaretAtMarkdownOffset(root: HTMLElement, offset: number): void {
  const selection = root.ownerDocument.getSelection();
  if (!selection) return;
  const range = root.ownerDocument.createRange();
  let left = offset;
  for (const child of Array.from(root.childNodes)) {
    const isText = child.nodeType === Node.TEXT_NODE;
    const isChip = child instanceof HTMLElement && child.classList.contains(CHIP_CLASS);
    const size = isText
      ? (child as Text).length
      : isChip
        ? serializeChip(child as HTMLElement).length
        : 0;
    if (left <= size) {
      if (isText) range.setStart(child, left);
      else if (left === 0) range.setStartBefore(child);
      else range.setStartAfter(child);
      range.collapse(true);
      selection.removeAllRanges();
      selection.addRange(range);
      return;
    }
    left -= size;
  }
  range.selectNodeContents(root);
  range.collapse(offset <= 0);
  selection.removeAllRanges();
  selection.addRange(range);
}

/** Delete from the caret to the start of the current line. A selection deletes only the selection. */
export function deleteComposerLineBackward(root: HTMLElement): void {
  const selection = root.ownerDocument.getSelection();
  if (!selection || selection.rangeCount === 0) return;
  const range = selection.getRangeAt(0);
  if (!root.contains(range.commonAncestorContainer)) return;

  const text = readComposerText(root);
  const start = markdownOffset(root, range.startContainer, range.startOffset);
  const end = selection.isCollapsed
    ? start
    : markdownOffset(root, range.endContainer, range.endOffset);
  const from = selection.isCollapsed ? text.lastIndexOf('\n', start - 1) + 1 : Math.min(start, end);
  const to = Math.max(start, end);
  if (from === to) return;

  writeComposerText(root, text.slice(0, from) + text.slice(to));
  placeCaretAtMarkdownOffset(root, from);
  root.dispatchEvent(new Event('input', { bubbles: true }));
}
