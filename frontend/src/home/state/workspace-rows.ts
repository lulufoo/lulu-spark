/**
 * How many rows each workspace of the Home sidebar gets.
 *
 * A workspace needs one title row plus one row per project. The rows left over after Chats,
 * Read Later and the account bar are shared out like this:
 *
 * - A workspace that needs no more than the equal share takes exactly what it needs; the rows
 *   it leaves go back to the others, and the equal share is computed again (max-min fairness).
 * - The rest each get the equal share; the few rows left over go one by one, in list order, to
 *   workspaces that are still cut off.
 * - A cut-off workspace with `a` rows shows `a - 1` projects under its title row and reports how
 *   many it hid (`hidden`); the caller shows that as a `…` mark on the title row, which takes
 *   no row of its own.
 * - If even the title rows do not fit, nothing is shared: every workspace shows its title row
 *   only and `overflow` tells the caller to let the area scroll.
 *
 * Every workspace except the first has a gap above it. A gap is a fraction of a row (the gap
 * height over the row height); the gaps are taken off the available height before it is counted
 * in whole rows, so the last workspace is never cut to a half-visible row.
 */

export type RowPlan = {
  /** Projects shown under the title row. */
  shown: number;
  /** Projects cut off, not shown; the title row marks them with `…`. */
  hidden: number;
};

export type RowAllocation = {
  plans: RowPlan[];
  /** The title rows alone do not fit in the available rows. */
  overflow: boolean;
};

/** Absorbs floating point noise, so a height that fits exactly is not counted one row short. */
const EPSILON = 1e-6;

function wholeRows(value: number): number {
  return Number.isFinite(value) && value > 0 ? Math.floor(value + EPSILON) : 0;
}

function gapFraction(value: number): number {
  return Number.isFinite(value) && value > 0 ? value : 0;
}

function shareRows(total: number, need: number[]): number[] {
  const rows: number[] = need.map(() => 0);
  let left = total;
  let open = need.map((_, i) => i);

  for (;;) {
    if (open.length === 0) return rows;
    const share = Math.floor(left / open.length);
    const satisfied = open.filter((i) => need[i] <= share);
    if (satisfied.length === 0) break;
    for (const i of satisfied) {
      rows[i] = need[i];
      left -= need[i];
    }
    open = open.filter((i) => need[i] > share);
  }

  const share = Math.floor(left / open.length);
  let spare = left - share * open.length;
  for (const i of open) {
    rows[i] = share + (spare > 0 ? 1 : 0);
    if (spare > 0) spare -= 1;
  }
  return rows;
}

/**
 * `availableRows` is the list height in rows (may be fractional); `projectCounts[i]` is the
 * project count of workspace `i`; `gapRows` is the gap above each workspace but the first, in
 * rows. The plans keep the workspace order.
 */
export function allocateWorkspaceRows(
  availableRows: number,
  projectCounts: number[],
  gapRows = 0,
): RowAllocation {
  const counts = projectCounts.map(wholeRows);
  if (counts.length === 0) return { plans: [], overflow: false };
  const total = wholeRows(availableRows - gapFraction(gapRows) * (counts.length - 1));
  if (total < counts.length) {
    return { plans: counts.map(() => ({ shown: 0, hidden: 0 })), overflow: true };
  }

  const need = counts.map((count) => 1 + count);
  const rows = shareRows(total, need);
  const plans = rows.map((given, i): RowPlan => {
    if (given >= need[i]) return { shown: counts[i], hidden: 0 };
    const shown = Math.max(0, given - 1);
    return { shown, hidden: counts[i] - shown };
  });
  return { plans, overflow: false };
}
