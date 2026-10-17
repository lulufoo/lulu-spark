import { describe, expect, it } from 'vitest';
import { dateKeyOf, formatDate, normalizeDateKey, timeFromTs, UNDATED_KEY } from '../../frontend/src/shared/utils.ts';
import {
  findNotesDateGroup,
  notesDateHeadingText,
} from '../../frontend/src/notes/commands/viewer/outlet.ts';
import { state } from '../../frontend/src/host/state.ts';

describe('Undated group key', () => {
  it('maps missing or malformed timestamps to the Undated key', () => {
    expect(dateKeyOf(undefined)).toBe(UNDATED_KEY);
    expect(dateKeyOf(null)).toBe(UNDATED_KEY);
    expect(dateKeyOf('')).toBe(UNDATED_KEY);
    expect(dateKeyOf('2025')).toBe(UNDATED_KEY);
    expect(dateKeyOf('202501011200')).toBe('20250101');
    expect(normalizeDateKey('undated')).toBe('undated');
    expect(normalizeDateKey('nope')).toBe('');
  });

  it('is not formatted as a date', () => {
    expect(formatDate(UNDATED_KEY)).toMatchObject({ label: 'Undated', full: 'Undated', year: '' });
    expect(timeFromTs('')).toBe('');
    expect(timeFromTs('202501011234')).toBe('12:34');
  });

  it('gets a heading and is found as a group like any date', () => {
    state.index.groupedByDate = [{ date: UNDATED_KEY, entries: [{ id: 'a', entry: {} }] }];
    state.index.filteredGroups = state.index.groupedByDate;
    expect(findNotesDateGroup(UNDATED_KEY)?.date).toBe(UNDATED_KEY);
    expect(notesDateHeadingText(UNDATED_KEY, findNotesDateGroup(UNDATED_KEY))).toBe('Undated  ·  1 items');
  });
});
