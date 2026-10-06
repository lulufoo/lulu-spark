import { completeAuthLogin } from '../auth/oauth.ts';
import { state } from '../host/state.ts';
import { openReadLaterDialog } from '../read-later/commands/dialog.ts';
import { navigateToNote } from './index.ts';
import {
  extractTraceFromScheme,
  logNotifyHop,
  parseTraceId,
  TRACE_PARAM,
  TRACE_QUERY,
} from './notify-trace.ts';

export type SparkEnvelope = {
  business: string;
  action: string;
  params: Record<string, unknown>;
};

export type ParsedSparkScheme =
  | { kind: 'notes-open'; id: string; path: string }
  | { kind: 'read-later-list' }
  | { kind: 'auth-login-callback'; code: string | null; error: string | null };

export type NotesLanding = { date: string; note: string };

function stringParam(params: Record<string, unknown>, key: string): string {
  const value = params[key];
  return typeof value === 'string' ? value : '';
}

export function composeSparkScheme(
  envelope: SparkEnvelope,
): string | null {
  const params = envelope.params;
  if (!params || typeof params !== 'object') return null;

  const trace = parseTraceId(stringParam(params, TRACE_PARAM));
  const traceQuery = trace ? `&${TRACE_QUERY}=${encodeURIComponent(trace)}` : '';

  if (envelope.business === 'notes' && (envelope.action === 'create' || envelope.action === 'update')) {
    const id = stringParam(params, 'id');
    const path = stringParam(params, 'common_path');
    if (!id || !path) return null;
    return `spark://notes/open?id=${encodeURIComponent(id)}&path=${encodeURIComponent(path)}${traceQuery}`;
  }

  if (envelope.business === 'read_later' && envelope.action === 'create') {
    return trace
      ? `spark://read-later/list?${TRACE_QUERY}=${encodeURIComponent(trace)}`
      : 'spark://read-later/list';
  }

  return null;
}

export function parseSparkScheme(scheme: string): ParsedSparkScheme | null {
  if (typeof scheme !== 'string' || !scheme.startsWith('spark://')) return null;

  let url: URL;
  try {
    url = new URL(scheme);
  } catch {
    return null;
  }

  if (url.protocol !== 'spark:') return null;
  const path = url.pathname.replace(/\/+$/, '');

  if (url.hostname === 'notes' && path === '/open') {
    const id = url.searchParams.get('id') ?? '';
    const notePath = url.searchParams.get('path') ?? '';
    if (!id || !notePath) return null;
    return { kind: 'notes-open', id, path: notePath };
  }

  if (url.hostname === 'read-later' && path === '/list') {
    for (const key of url.searchParams.keys()) {
      if (key !== TRACE_QUERY) return null;
    }
    return { kind: 'read-later-list' };
  }

  if (url.hostname === 'auth-login' && path === '/callback') {
    return {
      kind: 'auth-login-callback',
      code: url.searchParams.get('code') || null,
      error: url.searchParams.get('error') || null,
    };
  }

  return null;
}

export function resolveNotesLanding(id: string, path: string): NotesLanding | null {
  const data = state.index.data;
  if (!data) return null;

  const byId = id
    ? data[id] ?? Object.values(data).find((item) => item._id === id)
    : undefined;
  const entry = byId ?? Object.values(data).find((item) => path && item.common_path === path);

  const created = typeof entry?.created_at === 'string' ? entry.created_at : '';
  const note = typeof entry?.common_path === 'string' ? entry.common_path : '';
  if (created.length < 8 || !note) return null;
  return { date: created.slice(0, 8), note };
}

export function openSparkScheme(scheme: string): boolean {
  const trace = extractTraceFromScheme(scheme);
  const parsed = parseSparkScheme(scheme);
  if (!parsed) {
    logNotifyHop('route.to_business', trace, { outcome: 'parse_fail', scheme });
    return false;
  }

  if (parsed.kind === 'read-later-list') {
    openReadLaterDialog();
    logNotifyHop('route.to_business', trace, { outcome: 'ok', kind: 'read-later-list' });
    return true;
  }

  if (parsed.kind === 'auth-login-callback') {
    void completeAuthLogin(scheme);
    logNotifyHop('route.to_business', trace, { outcome: 'ok', kind: 'auth-login-callback' });
    return true;
  }

  const landing = resolveNotesLanding(parsed.id, parsed.path);
  if (!landing) {
    logNotifyHop('route.to_business', trace, {
      outcome: 'landing_miss',
      kind: 'notes-open',
      id: parsed.id,
      path: parsed.path,
    });
    return false;
  }
  const ok = navigateToNote({ date: landing.date, note: landing.note });
  logNotifyHop('route.to_business', trace, {
    outcome: ok ? 'ok' : 'nav_fail',
    kind: 'notes-open',
    id: parsed.id,
    path: parsed.path,
    date: landing.date,
    note: landing.note,
  });
  return ok;
}

function getTauriListen() {
  if (typeof window === 'undefined') return null;
  const listen = window.__TAURI__?.event?.listen;
  return typeof listen === 'function' ? listen : null;
}

let unlistenOpened: (() => void) | null = null;

export function startSparkSchemeOpenedHub(): void {
  if (typeof unlistenOpened === 'function') {
    unlistenOpened();
    unlistenOpened = null;
  }
  const listen = getTauriListen();
  if (!listen) return;
  void listen('spark-scheme:opened', (event) => {
    const payload = event?.payload;
    const scheme =
      typeof payload === 'string'
        ? payload
        : payload && typeof payload === 'object' && 'scheme' in payload
          ? (payload as { scheme?: unknown }).scheme
          : null;
    if (typeof scheme === 'string' && scheme) openSparkScheme(scheme);
  }).then((fn) => {
    unlistenOpened = fn;
  });
}
