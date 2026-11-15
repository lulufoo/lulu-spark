import type { BoardCacheInvalidated, BoardReadIdentity } from '../../host/api/board-sketch.ts';

export type ReplyOrigin = 'cache' | 'remote';
type DiagramMark = BoardReadIdentity & { denied: boolean; remoteSource: boolean; version?: number };
type ScopeMark = { generation: number; revoked: boolean };

export function compareIdentity(left: BoardReadIdentity, right: BoardReadIdentity): number {
  return left.generation - right.generation || left.diagram_generation - right.diagram_generation;
}

/** Watermarks advance only on a trusted source or an explicit access denial.
 * Network errors carry no evidence that a previous denial has been lifted. */
export class SketchFreshness {
  private generation = 0;
  private scopes = new Map<string, ScopeMark>();
  private diagrams = new Map<string, DiagramMark>();

  acceptSource(identity: BoardReadIdentity, origin: ReplyOrigin, version: number): boolean {
    if (identity.generation < this.generation) return false;
    const scope = this.scopes.get(identity.credential_scope);
    if (scope && (identity.generation < scope.generation
      || (identity.generation === scope.generation && scope.revoked))) return false;
    const key = this.key(identity);
    const diagram = this.diagrams.get(key);
    if (diagram) {
      const order = compareIdentity(identity, diagram);
      if (order < 0) return false;
      // A cache snapshot can observe the denied state while a remote recovery
      // is in progress. Its late reply cannot undo that remote's source.
      if (order === 0 && origin === 'cache'
        && (diagram.denied || (diagram.remoteSource && diagram.version !== version))) return false;
    }
    this.advanceGeneration(identity.generation);
    this.scopes.set(identity.credential_scope, { generation: identity.generation, revoked: false });
    this.diagrams.set(key, {
      ...identity, version, denied: false,
      remoteSource: origin === 'remote' || !!(diagram?.remoteSource && compareIdentity(identity, diagram) === 0),
    });
    return true;
  }

  acceptInvalidation(event: BoardCacheInvalidated, origin: ReplyOrigin | 'event'): boolean {
    const scope = this.scopes.get(event.credential_scope);
    if (scope && event.generation < scope.generation) return false;
    if (!event.diagram_id) {
      this.advanceGeneration(event.generation);
      this.scopes.set(event.credential_scope, { generation: event.generation, revoked: true });
      return true;
    }
    if (scope?.revoked && event.generation === scope.generation) return false;
    const identity = event as BoardReadIdentity;
    const previous = this.diagrams.get(this.key(identity));
    if (previous) {
      const order = compareIdentity(identity, previous);
      if (order < 0 || (order === 0 && origin === 'cache' && previous.remoteSource)) return false;
    }
    this.advanceGeneration(event.generation);
    this.scopes.set(event.credential_scope, { generation: event.generation, revoked: false });
    this.diagrams.set(this.key(identity), { ...identity, denied: true, remoteSource: false });
    return true;
  }

  private advanceGeneration(generation: number) {
    if (generation > this.generation) {
      this.scopes.clear();
      this.diagrams.clear();
    }
    this.generation = Math.max(this.generation, generation);
  }

  private key(identity: BoardReadIdentity): string {
    return JSON.stringify([identity.credential_scope, identity.diagram_id]);
  }
}
