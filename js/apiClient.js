/** Dev HTTP base; aligned with server.py / P0 lib.rs port 8765. */
export const DEFAULT_DEV_BASE = 'http://127.0.0.1:8765';

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
  const resolved = mode ?? envMode ?? 'fetch';
  if (arguments.length === 0) {
    return resolved;
  }
  if (resolved === 'fetch') {
    return createFetchDriver();
  }
  throw new Error(`Read driver "${resolved}" is not implemented`);
}
