// @vitest-environment jsdom
import { marked } from 'marked';
import { createElement } from 'react';
import { flushSync } from 'react-dom';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { releaseBoardSketches } from '../../frontend/src/board/commands/sketch-links.ts';
import { cleanupDocHighlightOverlay } from '../../frontend/src/doc-editor/highlights.ts';
import { state } from '../../frontend/src/notes/state/host.ts';
import { NotesCommentsBar } from '../../frontend/src/notes/ui/comments.tsx';
import { initHighlightUI, renderDocBody } from '../../frontend/src/notes/ui/viewer/body.tsx';
import { closeHighlightCommentEditor } from '../../frontend/src/notes/ui/viewer/highlight-comment.ts';
import { initTooltip, TIP_DELAY_MS } from '../../frontend/src/shared/tooltip/tooltip.ts';
import { CommentDeleteDialog } from '../../frontend/src/shared/comment-delete.tsx';

const host = vi.hoisted(() => ({
  fetchDocHighlights: vi.fn(),
  updateDocHighlights: vi.fn(),
  setDocHighlightComment: vi.fn(),
  updateComments: vi.fn(),
}));

vi.mock('../../frontend/src/host/api.ts', async (importOriginal) => ({
  ...(await importOriginal()),
  ...host,
}));

const id = 'a'.repeat(32);
const key = `doc:${id}`;
const markdown = 'Protect **handoff context** when teams grow.';
const selectedText = 'Protect handoff context when teams grow';
const savedHighlights = new Map();
const body = () => document.getElementById('md-body');
const fragments = () => Array.from(body().querySelectorAll('mark.doc-highlight'));
const editor = () => document.querySelector('.highlight-comment-editor');
const field = () => editor().querySelector('textarea');
let originalViewer;
let commentRoot;
let confirmationRoot;

beforeEach(() => {
  vi.resetAllMocks();
  vi.stubGlobal('marked', marked);
  savedHighlights.clear();
  originalViewer = { ...state.viewer };
  Object.assign(state.viewer, {
    entry: { id, _id: id },
    layer: 'raw',
    annotation: { raw: { comments: [{ id: 'overview', text: 'Whole-document comment' }] } },
    editing: false,
    loading: false,
    createSession: null,
  });
  document.body.innerHTML = `
    <div id="comments-host"></div>
    <div id="confirmation-host"></div>
    <div id="md-body"></div>
    <textarea id="md-edit-area" style="display:none"></textarea>
    <button id="highlight-add-btn" style="display:none">Highlight</button>
  `;
  confirmationRoot = createRoot(document.getElementById('confirmation-host'));
  flushSync(() => confirmationRoot.render(createElement(CommentDeleteDialog)));
  initHighlightUI();
  host.fetchDocHighlights.mockImplementation(async (docKey) => ({
    highlights: structuredClone(savedHighlights.get(docKey) ?? []),
  }));
  host.updateDocHighlights.mockImplementation(async (docKey, change) => {
    const highlights = savedHighlights.get(docKey) ?? [];
    if (change.id) {
      const highlight = highlights.find((record) => record.id === change.id);
      if (highlight?.comments?.length && !change.confirm_comment_removal) {
        return { ok: false, requires_confirmation: true };
      }
      savedHighlights.set(docKey, highlights.filter((highlight) => highlight.id !== change.id));
      return { ok: true, id: '' };
    }
    const highlight = { id: 'new-highlight', ...change, comments: [] };
    savedHighlights.set(docKey, [...highlights, highlight]);
    return { ok: true, id: highlight.id };
  });
  host.setDocHighlightComment.mockImplementation(async (docKey, highlightId, text) => {
    const highlight = savedHighlights.get(docKey)?.find(({ id }) => id === highlightId);
    if (!highlight) throw new Error('Highlight not found');
    const commentText = text.trim();
    const commentId = highlight.comments?.[0]?.id ?? 'saved-comment';
    highlight.comments = commentText ? [{ id: commentId, text: commentText }] : [];
    return { ok: true, comments: structuredClone(highlight.comments) };
  });
});

afterEach(() => {
  closeHighlightCommentEditor();
  cleanupDocHighlightOverlay();
  releaseBoardSketches(body());
  if (commentRoot) flushSync(() => commentRoot.unmount());
  commentRoot = undefined;
  flushSync(() => confirmationRoot.unmount());
  Object.assign(state.viewer, originalViewer);
  document.getSelection()?.removeAllRanges();
  document.body.replaceChildren();
  vi.unstubAllGlobals();
});

async function repaint() {
  state.viewer.bodyPaintKey += 1;
  await renderDocBody(markdown, 'raw', id);
}

async function saveComment(text) {
  field().value = text;
  editor().querySelector('[type="submit"]').click();
  await vi.waitFor(() => expect(editor()).toBeNull());
}

const confirmation = () => document.querySelector('#comment-delete-dialog.open');
const confirmRemoval = () => document.getElementById('btn-comment-delete-ok').click();

it('adds, restores, edits, cancels, clears and removes a comment on a legacy highlight', async () => {
  savedHighlights.set(key, [{ id: 'legacy-highlight', text: selectedText, occurrence: 0 }]);
  commentRoot = createRoot(document.getElementById('comments-host'));
  flushSync(() => commentRoot.render(createElement(NotesCommentsBar)));

  await repaint();
  await vi.waitFor(() => expect(fragments()).toHaveLength(3));
  expect(fragments().map((fragment) => fragment.dataset.hid))
    .toEqual(['legacy-highlight', 'legacy-highlight', 'legacy-highlight']);
  fragments()[0].click();
  expect(field().value).toBe('');
  expect(document.activeElement).toBe(field());
  await saveComment('Initial thought');
  expect(host.setDocHighlightComment).toHaveBeenLastCalledWith(key, 'legacy-highlight', 'Initial thought');
  const documentComments = document.getElementById('md-comments-bar');
  expect(documentComments.querySelectorAll('.comment-item')).toHaveLength(1);
  expect(documentComments.textContent).toContain('Whole-document comment');
  expect(documentComments.textContent).not.toContain('Initial thought');

  await repaint();
  await vi.waitFor(() => expect(fragments()).toHaveLength(3));
  expect(host.fetchDocHighlights.mock.calls).toEqual([[key], [key]]);
  fragments()[1].click();
  expect(field().value).toBe('Initial thought');
  expect(field().selectionStart).toBe('Initial thought'.length);
  await saveComment('Revised thought');

  fragments()[2].click();
  expect(field().value).toBe('Revised thought');
  field().value = 'Discard this';
  editor().querySelector('[type="button"]').click();
  expect(editor()).toBeNull();
  expect(host.setDocHighlightComment).toHaveBeenCalledTimes(2);
  fragments()[0].click();
  expect(field().value).toBe('Revised thought');
  await saveComment('');
  expect(host.setDocHighlightComment).toHaveBeenLastCalledWith(key, 'legacy-highlight', '');
  expect(fragments()).toHaveLength(3);

  fragments()[1].click();
  expect(field().value).toBe('');
  await saveComment('Remove with the highlight');
  const remove = body().querySelector('[aria-label="Remove highlight"]');
  expect(remove.textContent).toBe('×');
  remove.click();
  await vi.waitFor(() => expect(confirmation()?.textContent).toContain('saved comment'));
  document.getElementById('btn-comment-delete-cancel').click();
  await vi.waitFor(() => {
    expect(confirmation()).toBeNull();
    expect(document.getElementById('comment-delete-dialog-title').textContent).toBe('Delete comment');
  });
  expect(host.updateDocHighlights).not.toHaveBeenCalled();
  expect(fragments()).toHaveLength(3);
  expect(savedHighlights.get(key)[0].comments[0].text).toBe('Remove with the highlight');

  remove.click();
  await vi.waitFor(() => expect(confirmation()).not.toBeNull());
  const alert = vi.spyOn(window, 'alert').mockImplementation(() => {});
  try {
    host.updateDocHighlights.mockRejectedValueOnce(new Error('Storage busy'));
    confirmRemoval();
    await vi.waitFor(() => expect(alert).toHaveBeenCalledWith('Failed to remove highlight: Storage busy'));
    expect(fragments()).toHaveLength(3);
  } finally {
    alert.mockRestore();
  }
  host.updateDocHighlights.mockClear();
  remove.click();
  await vi.waitFor(() => expect(confirmation()).not.toBeNull());
  confirmRemoval();
  await vi.waitFor(() => expect(fragments()).toHaveLength(0));
  expect(editor()).toBeNull();
  expect(host.updateDocHighlights).toHaveBeenCalledExactlyOnceWith(
    key, { id: 'legacy-highlight', confirm_comment_removal: true }, expect.any(String),
  );
  expect(savedHighlights.get(key)).toEqual([]);
  expect(body().textContent.trim()).toBe('Protect handoff context when teams grow.');
  expect(documentComments.textContent).toContain('Whole-document comment');
  expect(host.updateComments).not.toHaveBeenCalled();
});

it('removes an uncommented highlight immediately and confirms if storage has a newer comment', async () => {
  savedHighlights.set(key, [{ id: 'legacy-highlight', text: selectedText, occurrence: 0 }]);
  await repaint();
  await vi.waitFor(() => expect(fragments()).toHaveLength(3));
  body().querySelector('.highlight-del-btn').click();
  await vi.waitFor(() => expect(fragments()).toHaveLength(0));
  expect(confirmation()).toBeNull();
  expect(host.updateDocHighlights).toHaveBeenLastCalledWith(
    key, { id: 'legacy-highlight' }, expect.any(String),
  );

  savedHighlights.set(key, [{ id: 'legacy-highlight', text: selectedText, occurrence: 0 }]);
  await repaint();
  await vi.waitFor(() => expect(fragments()).toHaveLength(3));
  savedHighlights.get(key)[0].comments = [{ id: 'new-comment', text: 'Saved in another window' }];
  body().querySelector('.highlight-del-btn').click();
  await vi.waitFor(() => expect(confirmation()).not.toBeNull());
  expect(host.updateDocHighlights).toHaveBeenLastCalledWith(
    key, { id: 'legacy-highlight' }, expect.any(String),
  );
  expect(savedHighlights.get(key)[0].comments[0].text).toBe('Saved in another window');
  confirmRemoval();
  await vi.waitFor(() => expect(fragments()).toHaveLength(0));
  expect(host.updateDocHighlights).toHaveBeenLastCalledWith(
    key, { id: 'legacy-highlight', confirm_comment_removal: true }, expect.any(String),
  );
});

it('opens the same editor by keyboard on a newly created highlight', async () => {
  await repaint();
  const paragraph = body().querySelector('p');
  const range = document.createRange();
  range.setStart(paragraph.firstChild, 0);
  range.setEnd(paragraph.lastChild, paragraph.lastChild.textContent.length - 1);
  range.getBoundingClientRect = () => new DOMRect(80, 100, 210, 18);
  document.getSelection().addRange(range);
  paragraph.dispatchEvent(new MouseEvent('mouseup', { bubbles: true }));
  const addHighlight = document.getElementById('highlight-add-btn');
  expect(addHighlight.style.display).toBe('block');
  addHighlight.click();

  await vi.waitFor(() => expect(fragments()).toHaveLength(3));
  expect(host.updateDocHighlights).toHaveBeenCalledExactlyOnceWith(
    key, { text: selectedText, occurrence: 0 }, expect.any(String),
  );
  const trigger = fragments()[0];
  expect(trigger.getAttribute('aria-haspopup')).toBe('dialog');
  trigger.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
  expect(field().value).toBe('');
  await saveComment('Written from the keyboard');
  expect(host.setDocHighlightComment).toHaveBeenCalledExactlyOnceWith(
    key, 'new-highlight', 'Written from the keyboard',
  );
});

it('previews saved comments on each fragment and drops the preview when editing or rerendering', async () => {
  const initial = 'Initial thought\n<b>Literal text</b>';
  savedHighlights.set(key, [{
    id: 'legacy-highlight',
    text: selectedText,
    occurrence: 0,
    comments: [{ id: 'comment-1', text: initial }],
  }]);
  const stopTooltip = initTooltip();
  const tip = document.querySelector('[role="tooltip"]');
  tip.getBoundingClientRect = () => new DOMRect(0, 0, 324, 95);
  const over = (target, point = {}) => target.dispatchEvent(new MouseEvent('mouseover', {
    bubbles: true,
    ...point,
  }));
  async function hoverPreview(target, text, point) {
    over(target, point);
    await vi.waitFor(() => {
      expect(tip.style.display).toBe('block');
      expect(tip.classList.contains('highlight-comment-popover')).toBe(true);
      expect(tip.classList.contains('highlight-comment-preview')).toBe(true);
      expect(tip.querySelector('h3')?.textContent).toBe('Highlight comment');
      expect(tip.querySelector('p.highlight-comment-preview__text')?.textContent).toBe(text);
    });
  }
  function expectNoPreview(target) {
    vi.useFakeTimers();
    try {
      over(target);
      vi.advanceTimersByTime(TIP_DELAY_MS);
      expect(tip.style.display).toBe('none');
    } finally {
      vi.useRealTimers();
    }
  }
  async function hoverRemoveButton() {
    over(fragments()[2].querySelector('.highlight-del-btn'));
    await vi.waitFor(() => {
      expect(tip.style.display).toBe('block');
      expect(tip.className).toBe('ui-tip');
      expect(tip.textContent).toBe('Remove highlight');
    });
  }

  try {
    await repaint();
    await vi.waitFor(() => expect(fragments()).toHaveLength(3));
    const wrapped = fragments()[0];
    const firstLine = new DOMRect(550, 200, 160, 20);
    const secondLine = new DOMRect(520, 224, 145, 20);
    wrapped.getClientRects = () => [firstLine, secondLine];
    const point = { clientX: 600, clientY: 230 };
    await hoverPreview(wrapped, initial, point);
    expect(tip.style.top).toBe(`${secondLine.bottom + 12}px`);
    const arrowOffset = parseFloat(tip.style.getPropertyValue('--highlight-comment-arrow'));
    expect(parseFloat(tip.style.left) + arrowOffset).toBeCloseTo(secondLine.right - 8);
    expect(tip.querySelector('b, form, textarea, button')).toBeNull();

    await hoverPreview(fragments()[1], initial);
    fragments()[1].dispatchEvent(new MouseEvent('mouseout', { bubbles: true, relatedTarget: body() }));
    expect(tip.style.display).toBe('none');
    await hoverPreview(wrapped, initial, point);
    wrapped.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, ...point }));
    expect(tip.style.display).toBe('none');
    expect(editor().classList.contains('highlight-comment-popover')).toBe(true);
    expect(editor().style.top).toBe(`${secondLine.bottom + 12}px`);
    expect(field().value).toBe(initial);
    expectNoPreview(fragments()[1]);
    await hoverRemoveButton();
    await saveComment('Revised thought');

    await hoverPreview(fragments()[2], 'Revised thought');
    await hoverRemoveButton();
    await hoverPreview(fragments()[0], 'Revised thought', point);
    await repaint();
    await vi.waitFor(() => expect(tip.style.display).toBe('none'));
    await vi.waitFor(() => expect(fragments()).toHaveLength(3));
    await hoverPreview(fragments()[2], 'Revised thought');
    fragments()[2].click();
    await saveComment('');
    expect(fragments().every((mark) => !mark.hasAttribute('data-tip'))).toBe(true);
    expectNoPreview(fragments()[2]);
  } finally {
    stopTooltip();
  }
});
