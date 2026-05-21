/**
 * Pre-fix reorder semantics for bug reproduction tests only (not used in app).
 */

import { swapAdjacent, validateCommentIdsForReorder } from './comment-reorder.js';

/** Old handler: optimistic swap + rollback on save failure */
export async function legacyMoveWithRollback(arr, idx, delta, save) {
  const before = arr.map((c) => c.id);
  if (!swapAdjacent(arr, idx, delta)) {
    return { before, afterOptimistic: before, afterFinal: before, rolledBack: false };
  }
  const afterOptimistic = arr.map((c) => c.id);
  try {
    await save(arr);
  } catch {
    swapAdjacent(arr, idx, delta);
    return {
      before,
      afterOptimistic,
      afterFinal: arr.map((c) => c.id),
      rolledBack: true,
    };
  }
  return {
    before,
    afterOptimistic,
    afterFinal: arr.map((c) => c.id),
    rolledBack: false,
  };
}

/** Current handler: optimistic swap, no rollback on failure */
export async function currentMoveNoRollback(arr, idx, delta, save) {
  const before = arr.map((c) => c.id);
  if (!swapAdjacent(arr, idx, delta)) {
    return { before, afterFinal: before, saveFailed: false };
  }
  try {
    await save(arr);
    return { before, afterFinal: arr.map((c) => c.id), saveFailed: false };
  } catch {
    return {
      before,
      afterFinal: arr.map((c) => c.id),
      saveFailed: true,
    };
  }
}

/** Old backend: filter_map ids without len check — drops unknown ids, still "ok" */
export function legacyReorderIds(comments, ids) {
  const map = new Map(comments.filter((c) => c.id).map((c) => [c.id, c]));
  return ids.map((id) => map.get(id)).filter(Boolean);
}

/** Old click path: map ids without filtering missing */
export function legacyIdsFromComments(comments) {
  return comments.map((c) => c.id);
}

export { validateCommentIdsForReorder };
