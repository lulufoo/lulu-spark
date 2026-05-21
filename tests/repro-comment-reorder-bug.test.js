/**
 * Bug repro: note ↑/↓ "no effect" — evidence via simulated pre-fix behavior.
 * Run: npm test -- tests/repro-comment-reorder-bug.test.js
 */
import { describe, expect, it } from 'vitest';
import {
  currentMoveNoRollback,
  legacyIdsFromComments,
  legacyMoveWithRollback,
  legacyReorderIds,
} from '../frontend/js/comment-reorder-legacy.js';
import { ensureLayerComments } from '../frontend/js/comment-reorder.js';

describe('repro: note reorder bug (simulated)', () => {
  it('✅ evidence A — save fail + rollback restores original order (looks like no-op)', async () => {
    const arr = [{ id: '1' }, { id: '2' }, { id: '3' }];
    const r = await legacyMoveWithRollback(arr, 1, -1, async () => {
      throw new Error('Invalid common_path');
    });
    expect(r.before).toEqual(['1', '2', '3']);
    expect(r.afterOptimistic).toEqual(['2', '1', '3']);
    expect(r.afterFinal).toEqual(['1', '2', '3']);
    expect(r.rolledBack).toBe(true);
  });

  it('✅ evidence A contrast — current: save fail keeps optimistic order', async () => {
    const arr = [{ id: '1' }, { id: '2' }, { id: '3' }];
    const r = await currentMoveNoRollback(arr, 1, -1, async () => {
      throw new Error('Invalid common_path');
    });
    expect(r.afterFinal).toEqual(['2', '1', '3']);
    expect(r.saveFailed).toBe(true);
  });

  it('✅ evidence B — legacy backend drops unknown id but "succeeds" with fewer items', () => {
    const comments = [
      { id: 'aaa', text: 'a' },
      { id: 'bbb', text: 'b' },
    ];
    const idsSent = legacyIdsFromComments(comments);
    expect(idsSent).toEqual(['aaa', 'bbb']);
    const persisted = legacyReorderIds(comments, ['bbb', 'nope', 'aaa']);
    expect(persisted.map((c) => c.id)).toEqual(['bbb', 'aaa']);
    expect(persisted).toHaveLength(2);
    // UI had 2 items; if ids included bad id, old Rust still returned ok:true with 2 items
    // but order wrong vs intent; with 3rd ghost id would drop to 2 only
  });

  it('✅ evidence B — missing comment id in payload (undefined in JSON)', () => {
    const comments = [{ id: 'aaa' }, { text: 'no id' }];
    const idsSent = legacyIdsFromComments(comments);
    expect(idsSent).toEqual(['aaa', undefined]);
    const persisted = legacyReorderIds(comments, idsSent.filter(Boolean));
    expect(persisted.map((c) => c.id)).toEqual(['aaa']);
    expect(persisted).toHaveLength(1);
  });

  it('⚠️ detached-array — only when layer missing; normal path shares state ref', () => {
    const ann = {};
    ensureLayerComments(ann, 'raw');
    ann.raw.comments.push({ id: 'a' }, { id: 'b' });
    const ldNew = ann.raw;
    const arrNew = ldNew.comments;
    arrNew.reverse();
    expect(ann.raw.comments.map((c) => c.id)).toEqual(['b', 'a']);

    const layer = 'raw';
    const ldOld = ann[layer] || {};
    const arrOld = ldOld.comments || [];
    expect(arrOld).toBe(ann.raw.comments);
  });

  it('✅ orphan ||{} pattern mutates throwaway array (layer absent)', () => {
    const ann = {};
    const layer = 'raw';
    const ld = ann[layer] || {};
    const arr = ld.comments || [];
    arr.push({ id: 'only' });
    expect(ann.raw).toBeUndefined();
  });
});
