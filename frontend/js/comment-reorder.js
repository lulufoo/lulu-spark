/**
 * Shared helpers for corpus/KB comment list reorder (in-memory + id validation).
 */

/** @param {Record<string, unknown>} annotation */
export function ensureLayerComments(annotation, layer) {
  if (!annotation[layer]) annotation[layer] = {};
  const ld = annotation[layer];
  if (!ld.comments) ld.comments = [];
  return ld.comments;
}

/** @param {Record<string, unknown>} annotation */
export function ensureKbComments(annotation) {
  if (!annotation.comments) annotation.comments = [];
  return annotation.comments;
}

/**
 * @param {Array<{ id?: string }>} comments
 * @param {number} idx
 * @param {number} delta -1 = up, +1 = down
 */
export function swapAdjacent(comments, idx, delta) {
  const j = idx + delta;
  if (j < 0 || j >= comments.length) return false;
  [comments[idx], comments[j]] = [comments[j], comments[idx]];
  return true;
}

/** @param {Array<{ id?: string }>} comments */
export function commentIdsForReorder(comments) {
  return comments
    .map((c) => c.id)
    .filter((id) => typeof id === 'string' && id.length > 0);
}

/** @param {Array<{ id?: string }>} comments */
export function validateCommentIdsForReorder(comments) {
  const ids = commentIdsForReorder(comments);
  if (ids.length !== comments.length) {
    return { ok: false, error: '部分笔记缺少 id，无法排序。请编辑保存后再试。' };
  }
  return { ok: true, ids };
}
