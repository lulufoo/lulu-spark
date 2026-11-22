// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  closeHighlightCommentEditor,
  openHighlightCommentEditor,
} from '../../frontend/src/notes/ui/viewer/highlight-comment.ts';

const popover = () => document.querySelector('.highlight-comment-editor');
const textarea = () => popover().querySelector('textarea');
const save = () => popover().querySelector('[type="submit"]');
const cancel = () => popover().querySelector('[type="button"]');

let mark;

beforeEach(() => {
  mark = document.createElement('mark');
  mark.tabIndex = 0;
  mark.textContent = 'Highlighted words';
  document.body.appendChild(mark);
});

afterEach(() => {
  closeHighlightCommentEditor();
  document.body.replaceChildren();
  vi.restoreAllMocks();
});

function open(text = '', onSave = vi.fn().mockResolvedValue(undefined)) {
  openHighlightCommentEditor({ anchor: mark, text, onSave });
  return onSave;
}

describe('highlight comment editor', () => {
  it('opens blank and focused, then prefills a saved comment with the caret at its end', () => {
    open();
    expect(popover().getAttribute('role')).toBe('dialog');
    expect(document.activeElement).toBe(textarea());
    expect(textarea().value).toBe('');
    expect(mark.getAttribute('aria-expanded')).toBe('true');
    closeHighlightCommentEditor();

    open('Another thought');
    expect(document.activeElement).toBe(textarea());
    expect(textarea().value).toBe('Another thought');
    expect(textarea().selectionStart).toBe(15);
    expect(textarea().selectionEnd).toBe(15);
  });

  it('cancels without saving and returns focus to the highlight', () => {
    const onSave = open();
    textarea().value = 'Discard this draft';
    cancel().click();
    expect(onSave).not.toHaveBeenCalled();
    expect(popover()).toBeNull();
    expect(document.activeElement).toBe(mark);
  });

  it('handles Escape without sending it to the Notes document handler', () => {
    const onViewerKeyDown = vi.fn();
    document.addEventListener('keydown', onViewerKeyDown, { once: true });
    try {
      open();
      textarea().dispatchEvent(new KeyboardEvent('keydown', {
        key: 'Escape',
        bubbles: true,
        cancelable: true,
      }));
      expect(popover()).toBeNull();
      expect(document.activeElement).toBe(mark);
      expect(onViewerKeyDown).not.toHaveBeenCalled();
    } finally {
      document.removeEventListener('keydown', onViewerKeyDown);
    }
  });

  it('dismisses on pointerdown outside and leaves the editor open when clicked inside', () => {
    const pointerDown = () => new MouseEvent('pointerdown', { bubbles: true });
    open();
    textarea().dispatchEvent(pointerDown());
    expect(popover()).not.toBeNull();
    document.body.dispatchEvent(pointerDown());
    expect(popover()).toBeNull();
  });

  it('keeps its draft when another fragment of the same highlight is clicked', () => {
    mark.className = 'doc-highlight is-commentable';
    mark.dataset.hid = 'h1';
    const otherFragment = mark.cloneNode(true);
    document.body.appendChild(otherFragment);
    const onSave = open('Saved value');
    textarea().value = 'Draft to keep';
    const originalEditor = popover();
    otherFragment.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true }));
    openHighlightCommentEditor({ anchor: otherFragment, text: 'Saved value', onSave });
    expect(popover()).toBe(originalEditor);
    expect(textarea().value).toBe('Draft to keep');
  });

  it('keeps the draft after a failed save and closes after retrying successfully', async () => {
    const onSave = vi.fn()
      .mockRejectedValueOnce(new Error('Highlight not found'))
      .mockResolvedValueOnce(undefined);
    open('Saved value', onSave);
    textarea().value = 'Edited comment';
    save().click();

    await vi.waitFor(() => expect(popover().querySelector('[role="alert"]').hidden).toBe(false));
    expect(popover().querySelector('[role="alert"]').textContent).toBe('Highlight not found');
    expect(textarea().value).toBe('Edited comment');
    expect(save().disabled).toBe(false);
    save().click();

    await vi.waitFor(() => expect(popover()).toBeNull());
    expect(onSave.mock.calls).toEqual([['Edited comment'], ['Edited comment']]);
    expect(document.activeElement).toBe(mark);
  });

  it('submits only once while saving and preserves a blank value to clear a saved comment', async () => {
    let completeSave;
    const onSave = vi.fn(() => new Promise((resolve) => { completeSave = resolve; }));
    open('Old comment', onSave);
    textarea().value = '';
    save().click();
    popover().querySelector('form').dispatchEvent(
      new Event('submit', { bubbles: true, cancelable: true }),
    );
    expect(onSave.mock.calls).toEqual([['']]);
    expect(save().disabled).toBe(true);
    completeSave();
    await vi.waitFor(() => expect(popover()).toBeNull());
  });

  it('flips above the clicked line and clamps horizontally at the viewport edge', () => {
    const first = new DOMRect(100, 150, 110, 18);
    const last = new DOMRect(window.innerWidth - 80, window.innerHeight - 40, 60, 18);
    vi.spyOn(mark, 'getClientRects').mockReturnValue([first, last]);
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue(
      new DOMRect(0, 0, 324, 169),
    );

    openHighlightCommentEditor({
      anchor: mark,
      point: { x: last.left + 12, y: last.top + 5 },
      text: '',
      onSave: vi.fn(),
    });

    expect(popover().classList.contains('is-above')).toBe(true);
    expect(popover().style.left).toBe(`${window.innerWidth - 324 - 8}px`);
    expect(popover().style.top).toBe(`${last.top - 169 - 12}px`);
  });

  it('closes the editor if its highlight is replaced or removed', async () => {
    open();
    mark.remove();
    await vi.waitFor(() => expect(popover()).toBeNull());
  });
});
