import {
  createApiClient,
  createChannel as createChannelFromClient,
  createFetchDriver,
  createTauriDriver,
  invoke as invokeCommand,
  resolveReadDriver,
} from '../apiClient.ts';
import { asRecord, type ApiDriver, type ServiceError } from '../api-types.ts';

function isTauriRuntime() {
  if (typeof window === 'undefined') return false;
  return Boolean(window.__TAURI__ || window.__TAURI_INTERNALS__);
}

function isLikelyExternalBrowserOnTauriDev() {
  if (typeof window === 'undefined') return false;
  const host = window.location.hostname;
  const isLocalDevHost = host === '127.0.0.1' || host === 'localhost';
  const isTauriDevPort = window.location.port === '1430' || window.location.port === '5173';
  return isLocalDevHost && isTauriDevPort && !isTauriRuntime();
}

function normalizeReadError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  const isFetchFailure = /Failed to fetch|NetworkError/i.test(message);
  if (isLikelyExternalBrowserOnTauriDev() && isFetchFailure) {
    return new Error('Opened Tauri Dev page in an external browser — return to the app window.');
  }
  return error instanceof Error ? error : new Error(message);
}

/** Resolve on each call: `tauri dev` loads the Vite page before `__TAURI__` exists at import time. */
export function getReadDriver(): ApiDriver {
  const mode = resolveReadDriver();
  return resolveReadDriver(mode);
}

function getReadApi() {
  return createApiClient(getReadDriver());
}

function resolveWriteDriver(): ApiDriver {
  const env =
    typeof import.meta !== 'undefined' && import.meta.env?.VITE_WRITE_API;
  if (env === 'fetch') return createFetchDriver();
  if (env === 'tauri') return createTauriDriver();
  return isTauriRuntime() ? createTauriDriver() : createFetchDriver();
}

function getWriteDriver() {
  return resolveWriteDriver();
}

export async function writePost(path: string, body?: unknown): Promise<any> {
  const res = await getWriteDriver().postJson(path, body);
  const payload = await res.json();
  return assertWritePayload(payload);
}

/** Tauri write commands return `{ error, _status }` without throwing — normalize here. */
export function assertWritePayload(payload: unknown): any {
  const rec = asRecord(payload);
  if (rec?.error) {
    const msg = typeof rec.error === 'string' ? rec.error : 'Request failed';
    const err = new Error(msg) as ServiceError;
    if (typeof rec._status === 'number') err.status = rec._status;
    throw err;
  }
  return payload;
}

/** Tauri read commands return `{ error, _status }` without throwing — normalize here. */
export function assertReadPayload(payload: unknown): any {
  const rec = asRecord(payload);
  if (rec?.error) {
    const msg = typeof rec.error === 'string' ? rec.error : 'Request failed';
    const err = new Error(msg) as ServiceError;
    if (typeof rec._status === 'number') err.status = rec._status;
    throw err;
  }
  return payload;
}

export async function readGet(pathAndQuery: string): Promise<any> {
  try {
    const payload = await getReadApi().getJson(pathAndQuery);
    return assertReadPayload(payload);
  } catch (error) {
    throw normalizeReadError(error);
  }
}

export async function invoke(cmd: string, args?: Record<string, unknown>) {
  return assertWritePayload(await invokeCommand(cmd, args));
}

export async function createChannel(onmessage?: (payload?: any) => void) {
  return createChannelFromClient(onmessage);
}

export { normalizeReadError };
