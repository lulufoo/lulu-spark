// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { applyCachedHighlights } from '../../frontend/src/doc-editor/highlights.ts';
import { fetchDocHighlights, updateDocHighlights } from '../../frontend/src/host/api.ts';
import { confirmRemoveHighlight } from '../../frontend/src/shared/comment-delete.tsx';

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchDocHighlights: vi.fn(),
  updateDocHighlights: vi.fn(),
}));
vi.mock('../../frontend/src/shared/comment-delete.tsx', () => ({
  confirmRemoveHighlight: vi.fn(async () => true),
}));

describe('comment actions on document highlights', () => {
  const key = 'doc:note';
  const highlight = {
    id: 'saved-mark',
    text: 'Alpha bravo charlie',
    comments: [{ id: 'saved-comment', text: 'Existing comment' }],
  };
  let body;
  let open;

  beforeEach(() => {
    document.body.innerHTML = '<div><p>Alpha <strong>bravo</strong> charlie</p></div>';
    body = document.body.firstElementChild;
    open = vi.fn();
    fetchDocHighlights.mockReset().mockResolvedValue({ highlights: [highlight] });
    updateDocHighlights.mockReset().mockResolvedValue({ ok: true });
  });

  afterEach(() => {
    document.body.replaceChildren();
  });

  it('opens the same comment from each fragment and keeps removal separate', async () => {
    await applyCachedHighlights({ bodyEl: body, identityKey: key, onOpenComment: open });
    const marks = [...body.querySelectorAll('mark.doc-highlight')];
    expect(marks).toHaveLength(3);
    expect(marks.filter((mark) => mark.tabIndex === 0)).toEqual([marks[0]]);
    for (const mark of marks) {
      mark.click();
      expect(open).toHaveBeenLastCalledWith(key, highlight, mark, { x: 0, y: 0 });
    }
    marks[0].dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    expect(open).toHaveBeenLastCalledWith(key, highlight, marks[0], undefined);
    fetchDocHighlights.mockResolvedValue({ highlights: [] });
    const remove = marks[2].querySelector('.highlight-del-btn');
    remove.click();
    expect(confirmRemoveHighlight).toHaveBeenCalledWith(remove);
    expect(open).toHaveBeenCalledTimes(4);
    await vi.waitFor(() => expect(body.querySelector('mark')).toBeNull());
    expect(updateDocHighlights).toHaveBeenCalledWith(
      key, { id: highlight.id, confirm_comment_removal: true }, expect.any(String),
    );
  });

  it('keeps a late restore from a previous document out of the current body', async () => {
    let resolvePrevious;
    fetchDocHighlights.mockImplementationOnce(() => new Promise((resolve) => {
      resolvePrevious = resolve;
    }));
    const previous = applyCachedHighlights({ bodyEl: body, identityKey: 'doc:previous', onOpenComment: open });
    await applyCachedHighlights({ bodyEl: body, identityKey: key, onOpenComment: open });
    resolvePrevious({ highlights: [{ id: 'old', text: 'Alpha' }] });
    await previous;

    expect([...body.querySelectorAll('mark')].map((mark) => mark.dataset.hid)).toEqual(
      [highlight.id, highlight.id, highlight.id],
    );
    body.querySelector('mark').click();
    expect(open).toHaveBeenLastCalledWith(key, highlight, expect.any(HTMLElement), { x: 0, y: 0 });
  });
});
