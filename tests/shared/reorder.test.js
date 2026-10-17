import { describe, expect, it } from 'vitest';
import { insertionIndex, moveItem, withDisabledInPlace } from '../../frontend/src/shared/reorder.ts';

describe('reorder rules', () => {
  it('counts the other rows above the pointer as the insertion index', () => {
    expect(insertionIndex([10, 30, 50], 5)).toBe(0);
    expect(insertionIndex([10, 30, 50], 31)).toBe(2);
    expect(insertionIndex([10, 30, 50], 99)).toBe(3);
    expect(insertionIndex([], 99)).toBe(0);
  });

  it('moves one id so that `at` other ids come before it', () => {
    expect(moveItem(['a', 'b', 'c'], 'a', 2)).toEqual(['b', 'c', 'a']);
    expect(moveItem(['a', 'b', 'c'], 'c', 0)).toEqual(['c', 'a', 'b']);
    expect(moveItem(['a', 'b', 'c'], 'b', 1)).toEqual(['a', 'b', 'c']);
    expect(moveItem(['a', 'b', 'c'], 'b', 99)).toEqual(['a', 'c', 'b']);
    expect(moveItem(['a', 'b'], 'x', 0)).toEqual(['a', 'b']);
  });

  it('puts the new enabled order into the places enabled entries held; disabled stay put', () => {
    const enabled = [
      { id: 'a', sortIndex: 0 },
      { id: 'c', sortIndex: 2 },
      { id: 'd', sortIndex: 3 },
    ];
    const disabled = [{ id: 'b', sortIndex: 1 }];
    expect(withDisabledInPlace(enabled, disabled, ['a', 'c', 'd'])).toEqual(['a', 'b', 'c', 'd']);
    expect(withDisabledInPlace(enabled, disabled, ['d', 'a', 'c'])).toEqual(['d', 'b', 'a', 'c']);
    expect(withDisabledInPlace(enabled, [], ['c', 'd', 'a'])).toEqual(['c', 'd', 'a']);
  });
});
