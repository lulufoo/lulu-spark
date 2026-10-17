// @vitest-environment node
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { allocateWorkspaceRows } from '../../frontend/src/home/state/workspace-rows.ts';

const used = (plan) => 1 + plan.shown + (plan.more ? 1 : 0);
const shownOf = (result) => result.plans.map((p) => p.shown);
const moreOf = (result) => result.plans.map((p) => p.more);

describe('allocateWorkspaceRows', () => {
  it('shares 20 rows between needs 3, 4, 10, 12 as 3, 4, 7, 6 rows', () => {
    const result = allocateWorkspaceRows(20, [2, 3, 9, 11]);
    expect(result.overflow).toBe(false);
    expect(shownOf(result)).toEqual([2, 3, 5, 4]);
    expect(moreOf(result)).toEqual([false, false, true, true]);
    expect(result.plans.map(used)).toEqual([3, 4, 7, 6]);
  });

  it('shows everything when everything fits', () => {
    const result = allocateWorkspaceRows(40, [2, 3, 9]);
    expect(shownOf(result)).toEqual([2, 3, 9]);
    expect(moreOf(result)).toEqual([false, false, false]);
  });

  it('cuts every workspace off when all need more than the equal share', () => {
    const result = allocateWorkspaceRows(12, [20, 20, 20]);
    expect(shownOf(result)).toEqual([2, 2, 2]);
    expect(moreOf(result)).toEqual([true, true, true]);
  });

  it('hands the left-over rows out one by one in list order', () => {
    // 11 rows over two needs of 21: 5 each, the single spare row goes to the first.
    const result = allocateWorkspaceRows(11, [20, 20]);
    expect(result.plans.map(used)).toEqual([6, 5]);
    expect(shownOf(result)).toEqual([4, 3]);
  });

  it('gives a workspace with no projects just its title row and leaves the rest to others', () => {
    const result = allocateWorkspaceRows(10, [0, 20]);
    expect(result.plans[0]).toEqual({ shown: 0, more: false });
    expect(result.plans[1]).toEqual({ shown: 7, more: true });
    expect(result.plans.map(used)).toEqual([1, 9]);
  });

  it('folds a cut-off workspace below three rows to its title and gives the rows back', () => {
    // 5 rows, needs 10 and 10: 3 + 2 rows (the spare row goes to the first). The second would
    // show nothing useful, so it folds to one row and the first takes the other four:
    // title + 2 projects + `...`.
    const result = allocateWorkspaceRows(5, [9, 9]);
    expect(result.plans[0]).toEqual({ shown: 2, more: true });
    expect(result.plans[1]).toEqual({ shown: 0, more: false });
  });

  it('keeps folding until nothing changes', () => {
    const result = allocateWorkspaceRows(7, [9, 9, 9]);
    expect(result.overflow).toBe(false);
    const rows = result.plans.map(used);
    expect(rows.reduce((a, b) => a + b, 0)).toBeLessThanOrEqual(7);
    for (const plan of result.plans) {
      if (plan.more) expect(plan.shown).toBeGreaterThanOrEqual(1);
    }
  });

  it('asks the caller to scroll when even the title rows do not fit', () => {
    const result = allocateWorkspaceRows(2, [5, 5, 5]);
    expect(result.overflow).toBe(true);
    expect(result.plans).toEqual([
      { shown: 0, more: false },
      { shown: 0, more: false },
      { shown: 0, more: false },
    ]);
  });

  it('is exactly full of titles when the rows equal the workspace count', () => {
    const result = allocateWorkspaceRows(3, [5, 5, 5]);
    expect(result.overflow).toBe(false);
    expect(shownOf(result)).toEqual([0, 0, 0]);
    expect(moreOf(result)).toEqual([false, false, false]);
  });

  it('handles an empty list, one workspace and zero rows', () => {
    expect(allocateWorkspaceRows(10, [])).toEqual({ plans: [], overflow: false });
    expect(allocateWorkspaceRows(0, [3])).toEqual({ plans: [{ shown: 0, more: false }], overflow: true });
    expect(allocateWorkspaceRows(6, [10]).plans[0]).toEqual({ shown: 4, more: true });
    expect(allocateWorkspaceRows(11, [10]).plans[0]).toEqual({ shown: 10, more: false });
  });

  it('treats negative and fractional input as whole non-negative numbers', () => {
    const result = allocateWorkspaceRows(10.9, [-3, 2.7, Number.NaN]);
    expect(result.overflow).toBe(false);
    expect(result.plans[0]).toEqual({ shown: 0, more: false });
    expect(result.plans[1]).toEqual({ shown: 2, more: false });
    expect(result.plans[2]).toEqual({ shown: 0, more: false });
  });

  it('never uses more rows than available, never shows more than exists, over a small range', () => {
    const values = [0, 1, 2, 3, 6, 11];
    const lists = [[]];
    for (const a of values) {
      lists.push([a]);
      for (const b of values) {
        lists.push([a, b]);
        for (const c of values) lists.push([a, b, c]);
      }
    }
    for (const counts of lists) {
      for (let total = 0; total <= 16; total += 1) {
        const result = allocateWorkspaceRows(total, counts);
        const label = `${total} rows for ${JSON.stringify(counts)}`;
        expect(result.plans, label).toHaveLength(counts.length);
        expect(result.overflow, label).toBe(total < counts.length);
        if (!result.overflow) {
          const rows = result.plans.reduce((sum, plan) => sum + used(plan), 0);
          expect(rows, label).toBeLessThanOrEqual(total);
        }
        result.plans.forEach((plan, i) => {
          expect(plan.shown, label).toBeLessThanOrEqual(counts[i]);
          if (plan.more) expect(plan.shown, label).toBeGreaterThanOrEqual(1);
          if (plan.more) expect(plan.shown, label).toBeLessThan(counts[i]);
        });
        const needAll = counts.reduce((sum, c) => sum + 1 + c, 0);
        if (total >= needAll) {
          result.plans.forEach((plan, i) => {
            expect(plan, label).toEqual({ shown: counts[i], more: false });
          });
        }
      }
    }
  });

  it('is a pure module: no React, no DOM', () => {
    const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
    const source = readFileSync(join(root, 'frontend/src/home/state/workspace-rows.ts'), 'utf8');
    expect(source).not.toMatch(/from 'react/);
    expect(source).not.toMatch(/\bdocument\b|\bwindow\b/);
  });
});
