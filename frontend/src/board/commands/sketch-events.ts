import type { BoardCacheInvalidated, BoardReadIdentity } from '../../host/api/board-sketch.ts';
import type { SketchAttachment } from '../state/sketch-attachment.ts';
import { compareIdentity, SketchFreshness, type ReplyOrigin } from '../state/sketch-freshness.ts';
import { isBoardInvalidation } from '../state/sketch-reply.ts';

export type InvalidationListener = (callback: (event: BoardCacheInvalidated) => void) => Promise<() => void>;
export type SketchOwner = { attachments: SketchAttachment[] };
export type SketchEventPool = {
  owners: Set<SketchOwner>;
  freshness: SketchFreshness;
  ready: Promise<void>;
  stop?: () => void;
};

const pools = new Map<InvalidationListener, SketchEventPool>();

export function invalidateSketchAttachments(
  pool: SketchEventPool,
  event: BoardCacheInvalidated,
  origin: ReplyOrigin | 'event',
): boolean {
  if (!isBoardInvalidation(event)) return false;
  // A new credential's source may arrive before the old credential's queued
  // event. That event can still remove old matching views without lowering the
  // current watermark or affecting the new credential's attachments.
  if (!pool.freshness.acceptInvalidation(event, origin)) return false;
  for (const owner of pool.owners) {
    for (const attachment of owner.attachments) {
      const identity = attachment.identity;
      if (!attachment.isLive() || !identity || identity.credential_scope !== event.credential_scope) continue;
      if (event.diagram_id) {
        if (identity.diagram_id !== event.diagram_id || compareIdentity(identity, event as BoardReadIdentity) > 0) continue;
        attachment.invalidate(event as BoardReadIdentity);
      } else if (identity.generation <= event.generation) {
        attachment.invalidate({ ...identity, generation: event.generation });
      }
    }
  }
  return true;
}

export function acquireSketchEvents(listener: InvalidationListener, owner: SketchOwner) {
  let pool = pools.get(listener);
  if (!pool) {
    pool = { owners: new Set(), freshness: new SketchFreshness(), ready: Promise.resolve() };
    const created = pool;
    pools.set(listener, created);
    created.ready = Promise.resolve().then(() => listener((event) => {
      if (created.owners.size) invalidateSketchAttachments(created, event, 'event');
    })).then((stop) => {
      if (created.owners.size) created.stop = stop;
      else stop();
    }).catch(() => {
      // A listener failure must not prevent either read. Responses still carry
      // access scopes and freshness identities for controlled application.
    });
  }
  const active = pool;
  active.owners.add(owner);
  return {
    pool: active,
    ready: active.ready,
    release() {
      active.owners.delete(owner);
      if (active.owners.size) return;
      if (pools.get(listener) === active) pools.delete(listener);
      active.stop?.();
      active.stop = undefined;
    },
  };
}
