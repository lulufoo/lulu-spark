import type { BoardCacheInvalidated, BoardReadIdentity } from '../../host/api/board-sketch.ts';
import type { SketchSource } from './sketch-attachment.ts';

type RecordValue = Record<string, unknown>;
type ParsedReply =
  | { kind: 'source'; source: SketchSource }
  | { kind: 'invalidation'; event: BoardCacheInvalidated | null }
  | { kind: 'transient' };

function isInteger(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
}

function readIdentity(value: RecordValue, diagramId: string): BoardReadIdentity | null {
  if (typeof value.credential_scope !== 'string' || !value.credential_scope
    || !isInteger(value.generation) || !isInteger(value.diagram_generation)
    || value.diagram_id !== diagramId) return null;
  return {
    credential_scope: value.credential_scope, generation: value.generation,
    diagram_id: diagramId, diagram_generation: value.diagram_generation,
  };
}

function decodeBase64(raw: unknown): string {
  if (typeof raw !== 'string' || !raw) return '';
  try {
    const binary = atob(raw);
    const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
    const html = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    return html.trim() ? html : '';
  } catch {
    return '';
  }
}

export function parseSketchReply(reply: unknown, diagramId: string): ParsedReply {
  if (!reply || typeof reply !== 'object') return { kind: 'transient' };
  const value = reply as RecordValue;
  const identity = readIdentity(value, diagramId);
  const status = value.status;
  if (status === 'sketch' || status === 'not_sketch' || status === 'invalid_source') {
    if (!identity || !isInteger(value.version) || value.version < 1 || value.version > 2147483647) {
      return { kind: 'transient' };
    }
    const html = status === 'sketch' ? decodeBase64(value.content_b64) : '';
    if (status === 'sketch' && !html) return { kind: 'transient' };
    return {
      kind: 'source', source: {
        status, identity, html, version: value.version,
        title: typeof value.title === 'string' ? value.title.trim() : '',
      },
    };
  }
  if ((status === 'forbidden' || status === 'not_found') && value.invalidation_scope === 'diagram') {
    return { kind: 'invalidation', event: identity ? { ...identity, reason: status } : null };
  }
  if ((status === 'unconfigured' || status === 'unauthorized') && value.invalidation_scope === 'credential') {
    const event = typeof value.credential_scope === 'string' && value.credential_scope
      && isInteger(value.generation)
      ? { credential_scope: value.credential_scope, generation: value.generation, reason: 'unauthorized' as const }
      : null;
    return { kind: 'invalidation', event };
  }
  return { kind: 'transient' };
}

export function isBoardInvalidation(value: BoardCacheInvalidated): boolean {
  if (!value || typeof value.credential_scope !== 'string' || !value.credential_scope
    || !isInteger(value.generation)) return false;
  return value.diagram_id === undefined
    || (typeof value.diagram_id === 'string' && isInteger(value.diagram_generation));
}
