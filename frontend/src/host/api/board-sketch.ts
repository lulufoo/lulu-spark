import { readGet } from './transport.ts';

export type BoardInvalidationScope = 'credential' | 'diagram';

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
};

export type BoardReadFailure = NoPreview & (
  | { status: 'unconfigured' | 'unauthorized'; invalidation_scope: 'credential' }
  | { status: 'forbidden' | 'not_found'; invalidation_scope: 'diagram' }
  | {
    status: 'not_board' | 'invalid_id' | 'unavailable' | 'network' | 'invalid_document';
    invalidation_scope?: never;
  }
);

/** A diagram invalidation applies to the original request's diagram ID/aliases;
 * a credential invalidation applies to its credential generation. Neither scope
 * authorizes a late response to affect a newer request or credential generation.
 */
export type BoardSketchReply = BoardPreviewReply | BoardReadFailure;
export type BoardRemoteReadReply = BoardSketchReply;

/** Contract for the next cache-only entry: no Board network request or wait.
 * A miss includes absent, unreadable, or discarded malformed cache. The caller
 * then performs a separate full remote read. This type adds no callable stub.
 */
export type BoardCacheOnlyReply =
  | BoardPreviewReply
  | ({ status: 'cache_miss'; invalidation_scope?: never } & NoPreview)
  | ({ status: 'unconfigured'; invalidation_scope: 'credential' } & NoPreview)
  | ({
    status: 'not_board' | 'invalid_id' | 'unavailable';
    invalidation_scope?: never;
  } & NoPreview);

/** Full remote read, independent of local-cache completion. Keep the original
 * URL as the cache identity. For a valid cached source, an equal version keeps
 * the existing preview; a different version uses this response without another
 * remote read. No timestamp or content digest is used for this comparison.
 * Invalid remote documents and transient failures do not invalidate good cache.
 */
export async function fetchBoardSketch(url: string): Promise<BoardRemoteReadReply> {
  return readGet(`/api/board-sketch?url=${encodeURIComponent(url)}`);
}
