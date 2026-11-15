import { readGet } from './transport.ts';

export type BoardInvalidationScope = 'credential' | 'diagram';

export type BoardReadIdentity = {
  credential_scope: string;
  generation: number;
  diagram_id: string;
  /** A freshness epoch for new remote reads and access invalidation. Changing
   * this value alone never requires repainting an equal content version. */
  diagram_generation: number;
};

export type BoardCacheInvalidated = {
  credential_scope: string;
  generation: number;
  reason: 'credential_changed' | 'credential_removed' | 'unauthorized' | 'forbidden' | 'not_found';
} & (
  | { diagram_id: string; diagram_generation: number }
  | { diagram_id?: never; diagram_generation?: never }
);

type NoPreview = {
  content_b64?: never;
  title?: never;
  version?: never;
};

/** Source metadata is valid, including when the current renderer cannot use it. */
export type BoardPreviewReply = (
  | { status: 'sketch'; content_b64: string }
  | { status: 'not_sketch' | 'invalid_source'; content_b64?: never }
) & {
  /** A positive PostgreSQL integer, 1 through 2147483647. */
  version: number;
  title?: string;
  invalidation_scope?: never;
} & BoardReadIdentity;

export type BoardReadFailure = NoPreview & (
  | { status: 'unconfigured' | 'unauthorized'; invalidation_scope: 'credential' }
  | { status: 'forbidden' | 'not_found'; invalidation_scope: 'diagram' }
  | {
    status: 'not_board' | 'invalid_id' | 'unavailable' | 'network' | 'invalid_document' | 'superseded';
    invalidation_scope?: never;
  }
) & Partial<BoardReadIdentity>;

/** A diagram invalidation applies to the original request's diagram ID/aliases;
 * a credential invalidation applies to its credential generation. Neither scope
 * authorizes a late response to affect a newer request or credential generation.
 */
export type BoardSketchReply = BoardPreviewReply | BoardReadFailure;
export type BoardRemoteReadReply = BoardSketchReply;

/** Cache-only read: no Board network request or wait for an in-flight fetch.
 * A miss includes absent, unreadable, or discarded malformed cache. The caller
 * then performs a separate full remote read. Access denial is scoped, even when
 * it was durably remembered by an earlier remote read or an earlier process.
 */
export type BoardCacheOnlyReply =
  | BoardPreviewReply
  | ({ status: 'cache_miss'; invalidation_scope?: never } & NoPreview & Partial<BoardReadIdentity>)
  | ({ status: 'unconfigured' | 'unauthorized'; invalidation_scope: 'credential' }
    & NoPreview & Partial<BoardReadIdentity>)
  | ({ status: 'forbidden' | 'not_found'; invalidation_scope: 'diagram' }
    & NoPreview & BoardReadIdentity)
  | ({
    status: 'not_board' | 'invalid_id' | 'unavailable';
    invalidation_scope?: never;
  } & NoPreview & Partial<BoardReadIdentity>);

/** Full remote read, independent of local-cache completion. Keep the original
 * URL as the cache identity. For a valid cached source, an equal version keeps
 * the existing preview; a different version uses this response without another
 * remote read. No timestamp or content digest is used for this comparison.
 * Invalid remote documents and transient failures do not invalidate good cache.
 */
export async function fetchBoardSketch(url: string): Promise<BoardRemoteReadReply> {
  return readGet(`/api/board-sketch?url=${encodeURIComponent(url)}`);
}

export async function fetchCachedBoardSketch(url: string): Promise<BoardCacheOnlyReply> {
  return readGet(`/api/board-sketch-cache?url=${encodeURIComponent(url)}`);
}
