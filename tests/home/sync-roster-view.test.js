import { describe, expect, it } from 'vitest';
import {
  foldGroup,
  GROUP_VISIBLE_ROWS,
  groupRoster,
  hasFailedDocs,
  rowHover,
} from '../../frontend/src/home/state/sync-roster-view.ts';

const entry = (overrides = {}) => ({
  id: '0'.repeat(32),
  name: 'Notes',
  op: 'upload',
  state: 'not_started',
  reason: null,
  local: true,
  ...overrides,
});
const idOf = (n) => n.toString(16).padStart(32, '0');

describe('groupRoster', () => {
  it('orders the groups Failed, In progress, Done, Not started', () => {
    const entries = [
      entry({ id: idOf(1), state: 'not_started' }),
      entry({ id: idOf(2), state: 'done' }),
      entry({ id: idOf(3), state: 'running' }),
      entry({ id: idOf(4), state: 'failed', reason: 'x' }),
    ];

    const groups = groupRoster(entries);

    expect(groups.map((g) => [g.key, g.label])).toEqual([
      ['failed', 'Failed'],
      ['running', 'In progress'],
      ['done', 'Done'],
      ['not_started', 'Not started'],
    ]);
  });

  it('leaves out every group with no document in it, Failed included', () => {
    expect(groupRoster([])).toEqual([]);
    const groups = groupRoster([entry({ state: 'done' }), entry({ id: idOf(2), state: 'not_started' })]);
    expect(groups.map((g) => g.key)).toEqual(['done', 'not_started']);
  });

  it('keeps the order the host gave inside a group, and loses and doubles no document', () => {
    const entries = [
      entry({ id: idOf(1), op: 'upload', state: 'done' }),
      entry({ id: idOf(2), op: 'upload', state: 'failed', reason: 'a' }),
      entry({ id: idOf(3), op: 'pull', state: 'done' }),
      entry({ id: idOf(4), op: 'pull', state: 'failed', reason: 'b' }),
      entry({ id: idOf(5), op: 'pull', state: 'running' }),
    ];

    const groups = groupRoster(entries);

    expect(groups.find((g) => g.key === 'done').entries.map((e) => e.id)).toEqual([idOf(1), idOf(3)]);
    expect(groups.find((g) => g.key === 'failed').entries.map((e) => e.id)).toEqual([idOf(2), idOf(4)]);
    expect(groups.flatMap((g) => g.entries).length).toBe(entries.length);
  });
});

describe('foldGroup', () => {
  const many = (n) => Array.from({ length: n }, (_, i) => entry({ id: idOf(i + 1) }));

  it('shows a group of up to the limit whole', () => {
    expect(GROUP_VISIBLE_ROWS).toBe(5);
    const out = foldGroup(many(5), false);
    expect(out.shown.length).toBe(5);
    expect(out.hidden).toBe(0);
    expect(foldGroup([], false)).toEqual({ shown: [], hidden: 0 });
  });

  it('shows the first rows of a longer group and counts the rest', () => {
    const out = foldGroup(many(12), false);
    expect(out.shown.map((e) => e.id)).toEqual(many(5).map((e) => e.id));
    expect(out.hidden).toBe(7);
  });

  it('shows all of a longer group once it is opened', () => {
    const out = foldGroup(many(12), true);
    expect(out.shown.length).toBe(12);
    expect(out.hidden).toBe(0);
  });
});

describe('rowHover', () => {
  it('gives the name and the id', () => {
    expect(rowHover(entry({ id: idOf(1), name: 'A long name of a note' }))).toBe(`A long name of a note\n${idOf(1)}`);
  });

  it('gives the reason of a failure in the words the host gave, and only for a failure', () => {
    const failed = entry({ id: idOf(2), name: 'Broken', state: 'failed', reason: 'could not pack document 6b65edcc…' });
    expect(rowHover(failed)).toBe(`Broken\n${idOf(2)}\ncould not pack document 6b65edcc…`);
    expect(rowHover({ ...failed, reason: null })).toBe(`Broken\n${idOf(2)}`);
    expect(rowHover({ ...failed, state: 'done' })).toBe(`Broken\n${idOf(2)}`);
  });
});

describe('hasFailedDocs', () => {
  it('is true when one document of the list failed', () => {
    expect(hasFailedDocs([entry({ state: 'done' }), entry({ id: idOf(2), state: 'failed', reason: 'x' })])).toBe(true);
    expect(hasFailedDocs([entry({ state: 'done' })])).toBe(false);
    expect(hasFailedDocs([])).toBe(false);
  });
});
