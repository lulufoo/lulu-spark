/** Dev HTTP base; aligned with server.py / P0 lib.rs port 8765. */
export const DEFAULT_DEV_BASE = 'http://127.0.0.1:8765';

import { resolveInvokeFromPath } from './readApiInvokeMap.js';

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
        return {
          ok: true,
          status: 200,
          json: async () => payload,
          text: async () => JSON.stringify(payload),
        };
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        return {
          ok: false,
          status: 500,
          json: async () => ({ error: message }),
          text: async () => JSON.stringify({ error: message }),
        };
      }
    },
  };
}

/**
 * @param {{ getJson(pathAndQuery: string): Promise<unknown> }} driver
 */
export function createApiClient(driver) {
  return {
    getJson(pathAndQuery) {
      return driver.getJson(pathAndQuery);
    },
  };
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
