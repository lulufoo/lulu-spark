import { describe, expect, it } from 'vitest';
import {
  commentIdsForReorder,
  ensureKbComments,
  ensureLayerComments,
  swapAdjacent,
  validateCommentIdsForReorder,
} from '../../frontend/src/shared/comment-reorder.ts';

describe('comment-reorder', () => {
  it('ensureLayerComments returns mutable array on annotation[layer]', () => {
    const ann = {};
    const a = ensureLayerComments(ann, 'raw');
    const b = ensureLayerComments(ann, 'raw');
    expect(a).toBe(b);
    a.push({ id: 'x', text: 't' });
    expect(ann.raw.comments).toHaveLength(1);
  });

  it('swapAdjacent exchanges neighbors', () => {
    const arr = [{ id: '1' }, { id: '2' }, { id: '3' }];
    expect(swapAdjacent(arr, 1, -1)).toBe(true);
    expect(arr.map((c) => c.id)).toEqual(['2', '1', '3']);
    expect(swapAdjacent(arr, 0, -1)).toBe(false);
  });

  it('validateCommentIdsForReorder rejects missing id', () => {
    const r = validateCommentIdsForReorder([{ id: 'a' }, { text: 'no id' }]);
    expect(r.ok).toBe(false);
    expect(r.error).toMatch(/lack an id/);
  });

  it('commentIdsForReorder collects string ids only', () => {
    expect(commentIdsForReorder([{ id: 'a' }, { id: '' }, { id: 'b' }])).toEqual(['a', 'b']);
  });

  it('ensureKbComments attaches comments on annotation root', () => {
    const ann = {};
    const c = ensureKbComments(ann);
    expect(c).toBe(ann.comments);
  });
});
