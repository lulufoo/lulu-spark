/**
 * Shared helpers for notes/KB comment list reorder (in-memory + id validation).
 */

import type { Annotation, CommentLike, LayerData } from './types.ts';

export function ensureLayerComments(annotation: Annotation, layer: string): CommentLike[] {
  if (!annotation[layer]) annotation[layer] = {};
  const ld = annotation[layer] as LayerData;
  if (!ld.comments) ld.comments = [];
  return ld.comments;
}

export function ensureKbComments(annotation: Annotation): CommentLike[] {
  if (!annotation.comments) annotation.comments = [];
  return annotation.comments as CommentLike[];
}

/** delta -1 = up, +1 = down */
export function swapAdjacent(comments: CommentLike[], idx: number, delta: number): boolean {
  const j = idx + delta;
  if (j < 0 || j >= comments.length) return false;
  [comments[idx], comments[j]] = [comments[j], comments[idx]];
  return true;
}

export function commentIdsForReorder(comments: CommentLike[]): string[] {
  return comments
    .map((c) => c.id)
    .filter((id): id is string => typeof id === 'string' && id.length > 0);
}

export function validateCommentIdsForReorder(
  comments: CommentLike[],
): { ok: false; error: string } | { ok: true; ids: string[] } {
  const ids = commentIdsForReorder(comments);
  if (ids.length !== comments.length) {
    return { ok: false, error: 'Some notes lack an id; cannot reorder. Edit and save, then retry.' };
  }
  return { ok: true, ids };
}
