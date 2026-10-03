import { logAppEvent } from '../host/app-log.ts';

export const TRACE_QUERY = 'trace';
export const TRACE_PARAM = 'trace_id';

const TRACE_RE = /^[A-Za-z0-9_-]{8,64}$/;

let lastTraceId: string | null = null;

export function parseTraceId(value: unknown): string | null {
  if (typeof value !== 'string') return null;
  const trimmed = value.trim();
  return TRACE_RE.test(trimmed) ? trimmed : null;
}

export function extractTraceFromScheme(scheme: string): string | null {
  try {
    return parseTraceId(new URL(scheme).searchParams.get(TRACE_QUERY));
  } catch {
    return null;
  }
}

export function rememberNotifyTrace(traceId: string | null): string {
  const id = parseTraceId(traceId);
  if (id) lastTraceId = id;
  return id ?? lastTraceId ?? 'trace_missing';
}

export function lastNotifyTrace(): string | null {
  return lastTraceId;
}

export function logNotifyHop(
  node: string,
  traceId: string | null,
  extra?: Record<string, unknown>,
): void {
  const id = rememberNotifyTrace(traceId);
  const params = { ...(extra ?? {}) };
  console.info('[app-log]', JSON.stringify({ business: 'os-notify', trace_id: id, event: node, ...params }));
  logAppEvent({
    business: 'os-notify',
    event: node,
    traceId: id,
    params,
  });
}
