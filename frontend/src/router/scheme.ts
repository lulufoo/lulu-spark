import { LOGIN_EVENT_EXCHANGE, LOGIN_EVENT_ROUTE, logLoginHop } from '../auth/hop.ts';
import { completeAuthLogin } from '../auth/oauth.ts';
import { encodePassBag, extractPassIdFromScheme, PASS_QUERY } from '../auth/pass.ts';
import { dateKeyOf } from '../shared/utils.ts';
import { state } from '../host/state.ts';
import { openReadLaterDialog } from '../read-later/commands/dialog.ts';
import { navigateToNote } from './index.ts';
import {
  businessFromSparkHost,
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
  | { kind: 'docs-open'; id: string }
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

  if (envelope.business === 'docs' && (envelope.action === 'create' || envelope.action === 'update')) {
    const id = stringParam(params, 'archive_id');
    if (!id) return null;
    return `spark://docs/open?id=${encodeURIComponent(id)}&${passQuery}`;
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

  if (url.hostname === 'docs' && path === '/open') {
    if (url.searchParams.has(TRACE_QUERY) || !extractPassIdFromScheme(scheme)) return null;
    const id = url.searchParams.get('id') ?? '';
    if (!id) return null;
    return { kind: 'docs-open', id };
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

export function resolveNotesLanding(id: string): NotesLanding | null {
  const data = state.index.data;
  if (!data || !id) return null;

  const entry = data[id] ?? Object.values(data).find((item) => item._id === id || item.id === id);

  if (!entry) return null;
  return { date: dateKeyOf(entry.created_at), note: id };
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
  const hop = extractPassIdFromScheme(scheme);
  const parsed = parseSparkScheme(scheme);
  if (!parsed) {
    if (isAuthLoginScheme(scheme)) {
      logLoginHop(LOGIN_EVENT_ROUTE, hop, {
        outcome: 'parse_fail',
        kind: 'auth-login-callback',
      });
      return false;
    }
    logNotifyHop('route.to_business', hop, {
      outcome: 'parse_fail',
      scheme,
    }, businessFromSparkHost(scheme));
    return false;
  }

  if (parsed.kind === 'read-later-list') {
    openReadLaterDialog();
    logNotifyHop('route.to_business', hop, {
      outcome: 'ok',
      kind: 'read-later-list',
    }, businessFromSparkHost(scheme));
    return true;
  }

  if (parsed.kind === 'auth-login-callback') {
    logLoginHop(LOGIN_EVENT_ROUTE, hop, { outcome: 'ok', kind: 'auth-login-callback' });
    void completeAuthLogin(scheme)
      .then((outcome) => {
        logLoginHop(LOGIN_EVENT_EXCHANGE, hop, { outcome });
      })
      .catch(() => {
        logLoginHop(LOGIN_EVENT_EXCHANGE, hop, { outcome: 'fail' });
      });
    return true;
  }

  const landing = resolveNotesLanding(parsed.id);
  const business = businessFromSparkHost(scheme);
  if (!landing) {
    logNotifyHop('route.to_business', hop, {
      outcome: 'landing_miss',
      kind: 'docs-open',
      id: parsed.id,
    }, business);
    return false;
  }
  const ok = navigateToNote({ date: landing.date, note: landing.note });
  logNotifyHop('route.to_business', hop, {
    outcome: ok ? 'ok' : 'nav_fail',
    kind: 'docs-open',
    id: parsed.id,
    date: landing.date,
    note: landing.note,
  }, business);
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
