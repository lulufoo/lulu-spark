import { LOGIN_EVENT_EXCHANGE, LOGIN_EVENT_ROUTE, logLoginHop } from '../auth/hop.ts';
import { completeAuthLogin } from '../auth/oauth.ts';
import { encodePassBag, extractPassIdFromScheme, PASS_QUERY } from '../auth/pass.ts';
import { state } from '../host/state.ts';
import { openReadLaterDialog } from '../read-later/commands/dialog.ts';
import { navigateToNote } from './index.ts';
import {
  logNotifyHop,
  parseTraceId,
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

function hopId(params: Record<string, unknown>): string | null {
  return parseTraceId(stringParam(params, 'id'));
}

export function composeSparkScheme(
  envelope: SparkEnvelope,
): string | null {
  const params = envelope.params;
  if (!params || typeof params !== 'object') return null;

  const hop = hopId(params);
  if (!hop) return null;
  const passQuery = `${PASS_QUERY}=${encodePassBag(hop)}`;

  if (envelope.business === 'notes' && (envelope.action === 'create' || envelope.action === 'update')) {
    const id = stringParam(params, 'archive_id');
    const path = stringParam(params, 'common_path');
    if (!id || !path) return null;
    return `spark://notes/open?id=${encodeURIComponent(id)}&path=${encodeURIComponent(path)}&${passQuery}`;
  }

  if (envelope.business === 'read_later' && envelope.action === 'create') {
    return `spark://read-later/list?${passQuery}`;
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
    if (url.searchParams.has(TRACE_QUERY) || !extractPassIdFromScheme(scheme)) return null;
    const id = url.searchParams.get('id') ?? '';
    const notePath = url.searchParams.get('path') ?? '';
    if (!id || !notePath) return null;
    return { kind: 'notes-open', id, path: notePath };
  }

  if (url.hostname === 'read-later' && path === '/list') {
    for (const key of url.searchParams.keys()) {
      if (key !== PASS_QUERY) return null;
    }
    if (!extractPassIdFromScheme(scheme)) return null;
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

function loginTraceId(scheme: string): string | null {
  return extractPassIdFromScheme(scheme);
}

function isAuthLoginScheme(scheme: string): boolean {
  try {
    const parsed = new URL(scheme);
    return parsed.protocol === 'spark:' && parsed.hostname === 'auth-login';
  } catch {
    return false;
  }
}

export function openSparkScheme(scheme: string): boolean {
  const parsed = parseSparkScheme(scheme);
  if (!parsed) {
    if (isAuthLoginScheme(scheme)) {
      logLoginHop(LOGIN_EVENT_ROUTE, loginTraceId(scheme), {
        outcome: 'parse_fail',
        kind: 'auth-login-callback',
      });
      return false;
    }
    logNotifyHop('route.to_business', extractPassIdFromScheme(scheme), {
      outcome: 'parse_fail',
      scheme,
    }, 'app');
    return false;
  }

  if (parsed.kind === 'read-later-list') {
    openReadLaterDialog();
    logNotifyHop('route.to_business', extractPassIdFromScheme(scheme), {
      outcome: 'ok',
      kind: 'read-later-list',
    }, 'read_later');
    return true;
  }

  if (parsed.kind === 'auth-login-callback') {
    const id = loginTraceId(scheme);
    logLoginHop(LOGIN_EVENT_ROUTE, id, { outcome: 'ok', kind: 'auth-login-callback' });
    void completeAuthLogin(scheme)
      .then((outcome) => {
        logLoginHop(LOGIN_EVENT_EXCHANGE, id, { outcome });
      })
      .catch(() => {
        logLoginHop(LOGIN_EVENT_EXCHANGE, id, { outcome: 'fail' });
      });
    return true;
  }

  const landing = resolveNotesLanding(parsed.id, parsed.path);
  const hop = extractPassIdFromScheme(scheme);
  if (!landing) {
    logNotifyHop('route.to_business', hop, {
      outcome: 'landing_miss',
      kind: 'notes-open',
      id: parsed.id,
      path: parsed.path,
    }, 'notes');
    return false;
  }
  const ok = navigateToNote({ date: landing.date, note: landing.note });
  logNotifyHop('route.to_business', hop, {
    outcome: ok ? 'ok' : 'nav_fail',
    kind: 'notes-open',
    id: parsed.id,
    path: parsed.path,
    date: landing.date,
    note: landing.note,
  }, 'notes');
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
