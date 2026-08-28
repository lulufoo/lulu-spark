/** Dev HTTP base for legacy `VITE_*_API=fetch` browser mode (optional). */
export const DEFAULT_DEV_BASE = 'http://127.0.0.1:8765';

import { resolveInvokeFromPath } from './readApiInvokeMap.ts';
import { resolveReindexInvoke } from './searchApiInvokeMap.ts';
import { resolveSyncInvoke } from './syncApiInvokeMap.ts';
import { resolveWriteInvoke } from './writeApiInvokeMap.ts';
import type { ApiDriver, InvokeResponse, JsonReader, TauriInvoke } from './api-types.ts';

type TauriChannelCtor = new (onmessage?: (payload: unknown) => void) => unknown;

let invokeFnPromise: Promise<TauriInvoke> | null = null;

function getGlobalTauriInvoke(): TauriInvoke | null {
  if (typeof window === 'undefined') return null;
  const invokeFromTauri =
    window.__TAURI__?.core?.invoke || window.__TAURI_INTERNALS__?.invoke;
  return typeof invokeFromTauri === 'function' ? invokeFromTauri : null;
}

async function loadTauriInvoke(): Promise<TauriInvoke> {
  const globalInvoke = getGlobalTauriInvoke();
  if (globalInvoke) {
    return globalInvoke;
  }
  if (!invokeFnPromise) {
    invokeFnPromise = import('@tauri-apps/api/core').then((m) => m.invoke as TauriInvoke);
  }
  return invokeFnPromise;
}

let channelCtorPromise: Promise<TauriChannelCtor> | null = null;

async function loadTauriChannel(): Promise<TauriChannelCtor> {
  const fromWindow = typeof window !== 'undefined' && window.__TAURI__?.core?.Channel;
  if (typeof fromWindow === 'function') return fromWindow as TauriChannelCtor;
  if (!channelCtorPromise) {
    channelCtorPromise = import('@tauri-apps/api/core').then((m) => m.Channel as TauriChannelCtor);
  }
  return channelCtorPromise;
}

/** Request-scoped Tauri Channel. Home must not import @tauri-apps/* itself. */
export async function createChannel(onmessage?: (payload?: any) => void) {
  const Channel = await loadTauriChannel();
  if (typeof Channel !== 'function') {
    throw new Error('Tauri Channel is not available');
  }
  return new Channel(onmessage);
}

function isTauriRuntime() {
  if (typeof window === 'undefined') return false;
  return Boolean(window.__TAURI__ || window.__TAURI_INTERNALS__);
}

/**
 * @param {string} [baseUrl]
 * @returns {{ getJson(pathAndQuery: string): Promise<unknown> }}
 */
export function buildReadUrl(pathAndQuery: string, baseUrl = DEFAULT_DEV_BASE) {
  const root = baseUrl.replace(/\/$/, '');
  const path = pathAndQuery.startsWith('/') ? pathAndQuery : `/${pathAndQuery}`;
  return `${root}${path}`;
}

/**
 * Wrap Tauri invoke JSON payload as fetch-like Response (P1 read / P2 write).
 * @param {unknown} payload
 */
export function wrapInvokePayload(payload: unknown): InvokeResponse {
  if (payload && typeof payload === 'object' && 'error' in payload && payload.error) {
    const rec = payload as { error?: unknown; _status?: unknown };
    const status = typeof rec._status === 'number' ? rec._status : 500;
    return {
      ok: false,
      status,
      json: async () => payload,
      text: async () => JSON.stringify(payload),
    };
  }
  return {
    ok: true,
    status: 200,
    json: async () => payload,
    text: async () => JSON.stringify(payload),
  };
}

export function createFetchDriver(baseUrl = DEFAULT_DEV_BASE): ApiDriver {
  const root = baseUrl.replace(/\/$/, '');
  return {
    buildUrl(pathAndQuery: string) {
      return buildReadUrl(pathAndQuery, root);
    },
    async getJson(pathAndQuery: string) {
      const res = await fetch(this.buildUrl!(pathAndQuery), { method: 'GET' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return res.json();
    },
    async fetchGet(pathAndQuery: string) {
      return fetch(this.buildUrl!(pathAndQuery), { method: 'GET' });
    },
    async postJson(path: string, body?: unknown) {
      const pathname = path.startsWith('/') ? path : `/${path}`;
      return fetch(this.buildUrl!(pathname), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body ?? {}),
      });
    },
  };
}

export function createTauriDriver(): ApiDriver {
  return {
    async getJson(pathAndQuery: string) {
      const resolved = resolveInvokeFromPath(pathAndQuery);
      const pathname = new URL(pathAndQuery, 'http://local').pathname;
      if (!resolved) {
        throw new Error(`No Tauri invoke mapping for ${pathname}`);
      }
      const invokeFn = await loadTauriInvoke();
      return invokeFn(resolved.cmd, resolved.args);
    },
    async fetchGet(pathAndQuery: string) {
      try {
        const payload = await this.getJson(pathAndQuery);
        return wrapInvokePayload(payload);
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        return wrapInvokePayload({ error: message, _status: 500 });
      }
    },
    async postJson(path: string, body?: unknown) {
      const rec = body && typeof body === 'object' ? (body as Record<string, unknown>) : {};
      const resolved = resolveWriteInvoke(path, rec) ?? resolveSyncInvoke(path, rec);
      const pathname = path.startsWith('/') ? path : `/${path}`;
      if (!resolved) {
        throw new Error(`No Tauri invoke mapping for POST ${pathname}`);
      }
      const invokeFn = await loadTauriInvoke();
      try {
        const payload = await invokeFn(resolved.cmd, resolved.args);
        return wrapInvokePayload(payload);
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        return wrapInvokePayload({ error: message, _status: 500 });
      }
    },
  };
}

/**
 * @param {{ getJson(pathAndQuery: string): Promise<unknown> }} driver
 */
/**
 * Direct Tauri invoke for P3 reindex commands (not readDriver).
 * @param {'reindexKnowledge'|'reindexWorkbench'|'reindexKbRepo'|'getReindexStatus'|'getReindexWorkbenchStatus'} key
 * @param {Record<string, unknown>} [payload]
 */
export async function invoke(cmd: string, args?: Record<string, unknown>) {
  const invokeFn = await loadTauriInvoke();
  return args === undefined ? invokeFn(cmd) : invokeFn(cmd, args);
}

export async function invokeSearch(key: string, payload?: Record<string, unknown>): Promise<any> {
  const resolved = resolveReindexInvoke(key, payload);
  if (!resolved) {
    throw new Error(`No reindex invoke mapping for ${key}`);
  }
  const invokeFn = await loadTauriInvoke();
  try {
    return await invokeFn(resolved.cmd, resolved.args);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    return { error: message };
  }
}

export function createApiClient(driver: JsonReader) {
  return {
    getJson(pathAndQuery: string) {
      return driver.getJson(pathAndQuery);
    },
  };
}

/**
 * No args: current read driver mode (`fetch` | `tauri`, from VITE_READ_API).
 * With mode: returns bound driver instance (fetch only in P1).
 * @param {string} [mode]
 */
export function resolveReadDriver(): string;
export function resolveReadDriver(mode: string): ApiDriver;
export function resolveReadDriver(mode?: string): string | ApiDriver {
  const envMode =
    typeof import.meta !== 'undefined' && import.meta.env?.VITE_READ_API;
  const resolved = mode ?? envMode ?? (isTauriRuntime() ? 'tauri' : 'fetch');
  if (arguments.length === 0) {
    return resolved;
  }
  if (resolved === 'fetch') {
    return createFetchDriver();
  }
  if (resolved === 'tauri') {
    return createTauriDriver();
  }
  throw new Error(`Read driver "${resolved}" is not implemented`);
}
