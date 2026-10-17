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
 * - A cut-off workspace with `a` rows shows `a - 2` projects and a `...` row. With fewer than
 *   three rows that would show nothing useful, so it folds to its title row and gives its rows
 *   back; this repeats until nothing changes.
 * - If even the title rows do not fit, nothing is shared: every workspace shows its title row
 *   only and `overflow` tells the caller to let the area scroll.
 */

export type RowPlan = {
  /** Projects shown under the title row. */
  shown: number;
  /** Whether a `...` row follows the shown projects. */
  more: boolean;
};

export type RowAllocation = {
  plans: RowPlan[];
  /** The title rows alone do not fit in the available rows. */
  overflow: boolean;
};

/** Title + one project + `...`: the smallest cut-off block worth showing. */
const MIN_CUT_OFF_ROWS = 3;

function wholeRows(value: number): number {
  return Number.isFinite(value) && value > 0 ? Math.floor(value) : 0;
}

/** One round of sharing; workspaces in `folded` already take exactly their title row. */
function shareRows(total: number, need: number[], folded: ReadonlySet<number>): number[] {
  const rows: number[] = need.map((_, i) => (folded.has(i) ? 1 : 0));
  let left = total - folded.size;
  let open = need.map((_, i) => i).filter((i) => !folded.has(i));

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

/** `projectCounts[i]` is the project count of workspace `i`; the plans keep that order. */
export function allocateWorkspaceRows(availableRows: number, projectCounts: number[]): RowAllocation {
  const counts = projectCounts.map(wholeRows);
  const total = wholeRows(availableRows);
  if (counts.length === 0) return { plans: [], overflow: false };
  if (total < counts.length) {
    return { plans: counts.map(() => ({ shown: 0, more: false })), overflow: true };
  }

  const need = counts.map((count) => 1 + count);
  const folded = new Set<number>();
  let rows = shareRows(total, need, folded);
  for (;;) {
    const cutOffTooSmall = rows
      .map((given, i) => (!folded.has(i) && given < need[i] && given < MIN_CUT_OFF_ROWS ? i : -1))
      .filter((i) => i >= 0);
    if (cutOffTooSmall.length === 0) break;
    for (const i of cutOffTooSmall) folded.add(i);
    rows = shareRows(total, need, folded);
  }

  const plans = rows.map((given, i): RowPlan => {
    if (folded.has(i)) return { shown: 0, more: false };
    if (given >= need[i]) return { shown: counts[i], more: false };
    return { shown: given - 2, more: true };
  });
  return { plans, overflow: false };
}
