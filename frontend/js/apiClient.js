/** Dev HTTP base for legacy `VITE_*_API=fetch` browser mode (optional). */
export const DEFAULT_DEV_BASE = 'http://127.0.0.1:8765';

import { resolveInvokeFromPath } from './readApiInvokeMap.js';
import { resolveReindexInvoke } from './searchApiInvokeMap.js';
import { resolveSyncInvoke } from './syncApiInvokeMap.js';
import { resolveWriteInvoke } from './writeApiInvokeMap.js';

let invokeFnPromise = null;

function getGlobalTauriInvoke() {
  if (typeof window === 'undefined') return null;
  const invokeFromTauri =
    window.__TAURI__?.core?.invoke || window.__TAURI_INTERNALS__?.invoke;
  return typeof invokeFromTauri === 'function' ? invokeFromTauri : null;
}

async function loadTauriInvoke() {
  const globalInvoke = getGlobalTauriInvoke();
  if (globalInvoke) {
    return globalInvoke;
  }
  if (!invokeFnPromise) {
    invokeFnPromise = import('@tauri-apps/api/core').then((m) => m.invoke);
  }
  return invokeFnPromise;
}

function isTauriRuntime() {
  if (typeof window === 'undefined') return false;
  return Boolean(window.__TAURI__ || window.__TAURI_INTERNALS__);
}

/**
 * @param {string} [baseUrl]
 * @returns {{ getJson(pathAndQuery: string): Promise<unknown> }}
 */
export function buildReadUrl(pathAndQuery, baseUrl = DEFAULT_DEV_BASE) {
  const root = baseUrl.replace(/\/$/, '');
  const path = pathAndQuery.startsWith('/') ? pathAndQuery : `/${pathAndQuery}`;
  return `${root}${path}`;
}

/**
 * Wrap Tauri invoke JSON payload as fetch-like Response (P1 read / P2 write).
 * @param {unknown} payload
 */
export function wrapInvokePayload(payload) {
  if (
    payload &&
    typeof payload === 'object' &&
    'error' in payload &&
    payload.error
  ) {
    const status =
      typeof payload._status === 'number' ? payload._status : 500;
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

export function createFetchDriver(baseUrl = DEFAULT_DEV_BASE) {
  const root = baseUrl.replace(/\/$/, '');
  return {
    buildUrl(pathAndQuery) {
      return buildReadUrl(pathAndQuery, root);
    },
    async getJson(pathAndQuery) {
      const res = await fetch(this.buildUrl(pathAndQuery), { method: 'GET' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return res.json();
    },
    async fetchGet(pathAndQuery) {
      return fetch(this.buildUrl(pathAndQuery), { method: 'GET' });
    },
    async postJson(path, body) {
      const pathname = path.startsWith('/') ? path : `/${path}`;
      return fetch(this.buildUrl(pathname), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body ?? {}),
      });
    },
    async putJson(path, body) {
      const pathname = path.startsWith('/') ? path : `/${path}`;
      return fetch(this.buildUrl(pathname), {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body ?? {}),
      });
    },
  };
}

export function createTauriDriver() {
  return {
    async getJson(pathAndQuery) {
      const resolved = resolveInvokeFromPath(pathAndQuery);
      const pathname = new URL(pathAndQuery, 'http://local').pathname;
      if (!resolved) {
        throw new Error(`No Tauri invoke mapping for ${pathname}`);
      }
      const invoke = await loadTauriInvoke();
      return invoke(resolved.cmd, resolved.args);
    },
    async fetchGet(pathAndQuery) {
      try {
        const payload = await this.getJson(pathAndQuery);
        return wrapInvokePayload(payload);
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        return wrapInvokePayload({ error: message, _status: 500 });
      }
    },
    async postJson(path, body) {
      const resolved = resolveWriteInvoke(path, body) ?? resolveSyncInvoke(path, body);
      const pathname = path.startsWith('/') ? path : `/${path}`;
      if (!resolved) {
        throw new Error(`No Tauri invoke mapping for POST ${pathname}`);
      }
      const invoke = await loadTauriInvoke();
      try {
        const payload = await invoke(resolved.cmd, resolved.args);
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
export async function invoke(cmd, args) {
  const invokeFn = await loadTauriInvoke();
  return args === undefined ? invokeFn(cmd) : invokeFn(cmd, args);
}

export async function invokeSearch(key, payload) {
  const resolved = resolveReindexInvoke(key, payload);
  if (!resolved) {
    throw new Error(`No reindex invoke mapping for ${key}`);
  }
  const invoke = await loadTauriInvoke();
  try {
    return await invoke(resolved.cmd, resolved.args);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    return { error: message };
  }
}

export function createApiClient(driver) {
  return {
    getJson(pathAndQuery) {
      return driver.getJson(pathAndQuery);
    },
  };
}

/**
 * Full-table overwrite of Host notes selection via Sidecar HTTP PUT.
 * @param {{ date: string|null, documents: { id: string, selected: boolean }[] }} snapshot
 */
export async function writeNotesSelectionSnapshot(snapshot) {
  const driver = createFetchDriver();
  const res = await driver.putJson('/api/notes-selection', snapshot);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
}

/** Leave Notes business: write empty snapshot `{ date: null, documents: [] }`. */
export async function clearNotesSelectionSnapshot() {
  return writeNotesSelectionSnapshot({ date: null, documents: [] });
}

/**
 * No args: current read driver mode (`fetch` | `tauri`, from VITE_READ_API).
 * With mode: returns bound driver instance (fetch only in P1).
 * @param {string} [mode]
 */
export function resolveReadDriver(mode) {
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
