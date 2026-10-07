import { extractPassIdFromScheme } from '../auth/pass.ts';
import { logAppEvent } from '../host/app-log.ts';

export const TRACE_QUERY = 'trace';
export const TRACE_PARAM = 'trace_id';

const TRACE_RE = /^[A-Za-z0-9_-]{8,64}$/;
const MISSING_HOP_ID = 'trace_missing';

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
  return id ?? lastTraceId ?? MISSING_HOP_ID;
}

export function lastNotifyTrace(): string | null {
  return lastTraceId;
}

export function businessFromSparkHost(
  scheme: string,
): 'notes' | 'read_later' | 'login' | 'app' {
  if (!extractPassIdFromScheme(scheme)) return 'app';
  try {
    const url = new URL(scheme);
    if (url.protocol !== 'spark:') return 'app';
    if (url.hostname === 'notes') return 'notes';
    if (url.hostname === 'read-later') return 'read_later';
    if (url.hostname === 'auth-login') return 'login';
    return 'app';
  } catch {
    return 'app';
  }
}

export function logNotifyHop(
  node: string,
  hopId: string | null,
  extra?: Record<string, unknown>,
  business: string = 'app',
): void {
  const parsed = parseTraceId(hopId);
  if (parsed) rememberNotifyTrace(parsed);
  const id = parsed ?? MISSING_HOP_ID;
  const hopBusiness = business || 'app';
  const params = { ...(extra ?? {}) };
  console.info('[app-log]', JSON.stringify({ business: hopBusiness, trace_id: id, event: node, ...params }));
  logAppEvent({
    business: hopBusiness,
    event: node,
    traceId: id,
    params,
  });
}
