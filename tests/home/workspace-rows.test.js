// @vitest-environment node
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { allocateWorkspaceRows } from '../../frontend/src/home/state/workspace-rows.ts';

const used = (plan) => 1 + plan.shown;
const shownOf = (result) => result.plans.map((p) => p.shown);
const hiddenOf = (result) => result.plans.map((p) => p.hidden);

describe('allocateWorkspaceRows', () => {
  it('shares 20 rows between needs 3, 4, 10, 12 as 3, 4, 7, 6 rows, with no row spent on a marker', () => {
    const result = allocateWorkspaceRows(20, [2, 3, 9, 11]);
    expect(result.overflow).toBe(false);
    expect(shownOf(result)).toEqual([2, 3, 6, 5]);
    expect(hiddenOf(result)).toEqual([0, 0, 3, 6]);
    expect(result.plans.map(used)).toEqual([3, 4, 7, 6]);
  });

  it('shows everything when everything fits', () => {
    const result = allocateWorkspaceRows(40, [2, 3, 9]);
    expect(shownOf(result)).toEqual([2, 3, 9]);
    expect(hiddenOf(result)).toEqual([0, 0, 0]);
  });

  it('cuts every workspace off when all need more than the equal share', () => {
    const result = allocateWorkspaceRows(12, [20, 20, 20]);
    expect(shownOf(result)).toEqual([3, 3, 3]);
    expect(hiddenOf(result)).toEqual([17, 17, 17]);
  });

  it('hands the left-over rows out one by one in list order', () => {
    // 11 rows over two needs of 21: 5 each, the single spare row goes to the first.
    const result = allocateWorkspaceRows(11, [20, 20]);
    expect(result.plans.map(used)).toEqual([6, 5]);
    expect(shownOf(result)).toEqual([5, 4]);
    expect(hiddenOf(result)).toEqual([15, 16]);
  });

  it('gives a workspace with no projects just its title row and leaves the rest to others', () => {
    const result = allocateWorkspaceRows(10, [0, 20]);
    expect(result.plans[0]).toEqual({ shown: 0, hidden: 0 });
    expect(result.plans[1]).toEqual({ shown: 8, hidden: 12 });
    expect(result.plans.map(used)).toEqual([1, 9]);
  });

  it('never folds a cut-off workspace: with two rows it still shows one project', () => {
    // 5 rows, needs 10 and 10: 3 + 2 rows, the spare row goes to the first.
    const result = allocateWorkspaceRows(5, [9, 9]);
    expect(result.plans[0]).toEqual({ shown: 2, hidden: 7 });
    expect(result.plans[1]).toEqual({ shown: 1, hidden: 8 });
  });

  it('shows only the title, with everything counted as hidden, when a workspace gets one row', () => {
    const result = allocateWorkspaceRows(3, [9, 9, 9]);
    expect(result.overflow).toBe(false);
    expect(result.plans).toEqual([
      { shown: 0, hidden: 9 },
      { shown: 0, hidden: 9 },
      { shown: 0, hidden: 9 },
    ]);
  });

  it('asks the caller to scroll when even the title rows do not fit, and shows no badge', () => {
    const result = allocateWorkspaceRows(2, [5, 5, 5]);
    expect(result.overflow).toBe(true);
    expect(result.plans).toEqual([
      { shown: 0, hidden: 0 },
      { shown: 0, hidden: 0 },
      { shown: 0, hidden: 0 },
    ]);
  });

  it('handles an empty list, one workspace and zero rows', () => {
    expect(allocateWorkspaceRows(10, [])).toEqual({ plans: [], overflow: false });
    expect(allocateWorkspaceRows(0, [3])).toEqual({ plans: [{ shown: 0, hidden: 0 }], overflow: true });
    expect(allocateWorkspaceRows(6, [10]).plans[0]).toEqual({ shown: 5, hidden: 5 });
    expect(allocateWorkspaceRows(11, [10]).plans[0]).toEqual({ shown: 10, hidden: 0 });
  });

  it('treats negative and fractional input as whole non-negative numbers', () => {
    const result = allocateWorkspaceRows(10.9, [-3, 2.7, Number.NaN]);
    expect(result.overflow).toBe(false);
    expect(result.plans[0]).toEqual({ shown: 0, hidden: 0 });
    expect(result.plans[1]).toEqual({ shown: 2, hidden: 0 });
    expect(result.plans[2]).toEqual({ shown: 0, hidden: 0 });
  });

  it('never uses more rows than available, shown + hidden is always the project count, over a small range', () => {
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
          result.plans.forEach((plan, i) => expect(plan.shown + plan.hidden, label).toBe(counts[i]));
        } else {
          result.plans.forEach((plan) => expect(plan, label).toEqual({ shown: 0, hidden: 0 }));
        }
        const needAll = counts.reduce((sum, c) => sum + 1 + c, 0);
        if (total >= needAll) {
          result.plans.forEach((plan, i) => {
            expect(plan, label).toEqual({ shown: counts[i], hidden: 0 });
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
