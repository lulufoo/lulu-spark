import {
  createApiClient,
  createChannel as createChannelFromClient,
  createFetchDriver,
  createTauriDriver,
  invoke as invokeCommand,
  resolveReadDriver,
} from '../apiClient.js';

function isTauriRuntime() {
  if (typeof window === 'undefined') return false;
  return Boolean(window.__TAURI__ || window.__TAURI_INTERNALS__);
}

function isLikelyExternalBrowserOnTauriDev() {
  if (typeof window === 'undefined') return false;
  const host = window.location.hostname;
  const isLocalDevHost = host === '127.0.0.1' || host === 'localhost';
  return isLocalDevHost && window.location.port === '1430' && !isTauriRuntime();
}

function normalizeReadError(error) {
  const message = error instanceof Error ? error.message : String(error);
  const isFetchFailure = /Failed to fetch|NetworkError/i.test(message);
  if (isLikelyExternalBrowserOnTauriDev() && isFetchFailure) {
    return new Error('Opened Tauri Dev page in an external browser — return to the app window.');
  }
  return error instanceof Error ? error : new Error(message);
}

/** Resolve on each call: `tauri dev` loads page from :1430 before `__TAURI__` exists at import time. */
export function getReadDriver() {
  const mode = resolveReadDriver();
  return resolveReadDriver(mode);
}

function getReadApi() {
  return createApiClient(getReadDriver());
}

function resolveWriteDriver() {
  const env =
    typeof import.meta !== 'undefined' && import.meta.env?.VITE_WRITE_API;
  if (env === 'fetch') return createFetchDriver();
  if (env === 'tauri') return createTauriDriver();
  return isTauriRuntime() ? createTauriDriver() : createFetchDriver();
}

function getWriteDriver() {
  return resolveWriteDriver();
}

export async function writePost(path, body) {
  const res = await getWriteDriver().postJson(path, body);
  const payload = await res.json();
  return assertWritePayload(payload);
}

/** Tauri write commands return `{ error, _status }` without throwing — normalize here. */
export function assertWritePayload(payload) {
  if (payload && typeof payload === 'object' && payload.error) {
    const msg = typeof payload.error === 'string' ? payload.error : 'Request failed';
    const err = new Error(msg);
    if (typeof payload._status === 'number') err.status = payload._status;
    throw err;
  }
  return payload;
}

/** Tauri read commands return `{ error, _status }` without throwing — normalize here. */
export function assertReadPayload(payload) {
  if (payload && typeof payload === 'object' && payload.error) {
    const msg = typeof payload.error === 'string' ? payload.error : 'Request failed';
    const err = new Error(msg);
    if (typeof payload._status === 'number') err.status = payload._status;
    throw err;
  }
  return payload;
}

export async function readGet(pathAndQuery) {
  try {
    const payload = await getReadApi().getJson(pathAndQuery);
    return assertReadPayload(payload);
  } catch (error) {
    throw normalizeReadError(error);
  }
}

export async function invoke(cmd, args) {
  return assertWritePayload(await invokeCommand(cmd, args));
}

export async function createChannel(onmessage) {
  return createChannelFromClient(onmessage);
}

export { normalizeReadError };
