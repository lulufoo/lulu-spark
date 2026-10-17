/** Pure rules of drag-to-reorder lists; the pointer handling lives in `reorder-rows.tsx`. */

/** Where a dragged row would be inserted among the other rows: how many of them lie above `y`. */
export function insertionIndex(otherMidpoints: readonly number[], y: number): number {
  return otherMidpoints.filter((mid) => mid < y).length;
}

/** `ids` with `id` taken out and put back so that `at` other entries come before it. */
export function moveItem(ids: readonly string[], id: string, at: number): string[] {
  const others = ids.filter((x) => x !== id);
  if (!ids.includes(id)) return [...ids];
  others.splice(Math.max(0, Math.min(at, others.length)), 0, id);
  return others;
}

type Ranked = { id: string; sortIndex: number };

/**
 * The whole list for the backend after the enabled entries were reordered. Disabled entries are
 * not shown and cannot be dragged, so they keep their place: the enabled entries are put, in
 * their new order, into the places enabled entries held.
 */
export function withDisabledInPlace(
  enabled: readonly Ranked[],
  disabled: readonly Ranked[],
  newEnabledOrder: readonly string[],
): string[] {
  const all = [...enabled.map((r) => ({ ...r, on: true })), ...disabled.map((r) => ({ ...r, on: false }))].sort(
    (a, b) => a.sortIndex - b.sortIndex,
  );
  const queue = [...newEnabledOrder];
  return all.map((entry) => (entry.on ? (queue.shift() ?? entry.id) : entry.id));
}
