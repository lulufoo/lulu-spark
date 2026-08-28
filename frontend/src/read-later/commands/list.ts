import { createApiClient, resolveReadDriver } from '../../host/apiClient.ts';

function getTauriInvoke() {
  if (typeof window === 'undefined') return null;
  const invoke =
    window.__TAURI__?.core?.invoke || window.__TAURI_INTERNALS__?.invoke;
  return typeof invoke === 'function' ? invoke : null;
}

function getTauriOpener() {
  if (typeof window === 'undefined') return null;
  const opener = window.__TAURI__?.opener;
  const openUrl = opener?.openUrl;
  return typeof openUrl === 'function' ? openUrl.bind(opener) : null;
}

function serviceError(data: unknown) {
  if (!data || typeof data !== 'object' || Array.isArray(data)) return null;
  const rec = data as { error?: unknown; _status?: number };
  if (!rec.error) return null;
  const err = new Error(String(rec.error)) as Error & { status?: number };
  err.status = typeof rec._status === 'number' ? rec._status : 500;
  return err;
}

function httpStatusError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  const match = message.match(/^HTTP (\d+)/);
  if (!match) return null;
  const err = new Error(message) as Error & { status?: number };
  err.status = Number(match[1]);
  return err;
}

export async function openExternalUrl(url: string) {
  const openUrl = getTauriOpener();
  if (!openUrl) {
    throw new Error('Tauri opener unavailable');
  }
  await openUrl(url);
}

export async function loadReadLaterEntries() {
  const mode = resolveReadDriver();
  const client = createApiClient(resolveReadDriver(mode));
  try {
    const data = await client.getJson('/api/read-later');
    const unavailable = serviceError(data);
    if (unavailable) throw unavailable;
    return Array.isArray(data) ? data : [];
  } catch (error) {
    const httpErr = httpStatusError(error);
    if (httpErr) throw httpErr;
    throw error;
  }
}

export async function markEntryRead(id: string) {
  const invoke = getTauriInvoke();
  if (!invoke) {
    throw new Error('Tauri invoke unavailable');
  }
  const result = await invoke('mark_read_later', { id, read: true });
  const unavailable = serviceError(result);
  if (unavailable) throw unavailable;
}

export async function deleteReadLaterEntry(id: string) {
  const invoke = getTauriInvoke();
  if (!invoke) {
    throw new Error('Tauri invoke unavailable');
  }
  const result = await invoke('delete_read_later', { id });
  const unavailable = serviceError(result);
  if (unavailable) throw unavailable;
}

export function bindFocusRefresh(refresh: () => unknown) {
  const runRefresh = () => {
    void refresh();
  };
  const onVisibility = () => {
    if (document.visibilityState === 'visible') {
      runRefresh();
    }
  };
  window.addEventListener('focus', runRefresh);
  document.addEventListener('visibilitychange', onVisibility);
  return () => {
    window.removeEventListener('focus', runRefresh);
    document.removeEventListener('visibilitychange', onVisibility);
  };
}
