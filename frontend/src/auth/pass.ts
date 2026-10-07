import { parseTraceId } from '../router/notify-trace.ts';

export const PASS_QUERY = 'pass';

export function newLoginTraceId(): string {
  const bytes = new Uint8Array(6);
  crypto.getRandomValues(bytes);
  const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
  return `trace_${hex}`;
}

export function encodePassBag(id: string): string {
  return encodeURIComponent(JSON.stringify({ id }));
}

export function decodePassBag(raw: string | null | undefined): string | null {
  if (!raw) return null;
  const parsed = parseBagJson(raw) ?? parseBagJson(safeDecode(raw));
  return parseTraceId(parsed && typeof parsed === 'object' ? (parsed as { id?: unknown }).id : null);
}

export function extractPassIdFromScheme(scheme: string): string | null {
  try {
    return decodePassBag(new URL(scheme).searchParams.get(PASS_QUERY));
  } catch {
    return null;
  }
}

function safeDecode(raw: string): string {
  try {
    return decodeURIComponent(raw);
  } catch {
    return raw;
  }
}

function parseBagJson(raw: string): unknown {
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}
