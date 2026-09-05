import * as api from '../../host/api.ts';
import { indexRebuildStore, type IndexRebuildStatus } from '../state/index-rebuild.ts';

const POLL_MS = 2000;
let pollTimer: ReturnType<typeof setInterval> | null = null;

type StatusPayload = { status?: string; log?: string; error?: string };

function normalizeStatus(raw: unknown): IndexRebuildStatus {
  return raw === 'running' || raw === 'done' || raw === 'error' ? raw : 'idle';
}

function stopPolling() {
  if (pollTimer) clearInterval(pollTimer);
  pollTimer = null;
}

function applyStatus(res: StatusPayload) {
  if (res.error) {
    indexRebuildStore.set({ status: 'error', log: res.error });
    stopPolling();
    return;
  }
  const status = normalizeStatus(res.status);
  indexRebuildStore.set({ status, log: res.log || '' });
  if (status !== 'running') stopPolling();
}

export async function pollIndexRebuildStatus() {
  try {
    const res = (await api.getReindexAllStatus()) as StatusPayload;
    applyStatus(res);
  } catch {
    /* keep polling; the next tick may succeed */
  }
}

function ensurePolling() {
  if (pollTimer) return;
  pollTimer = setInterval(() => void pollIndexRebuildStatus(), POLL_MS);
}

/** Backend rebuilds on launch; pick up that job so the header spinner shows it. */
export async function watchIndexRebuildOnBoot() {
  await pollIndexRebuildStatus();
  if (indexRebuildStore.getSnapshot().status === 'running') ensurePolling();
}

/** Header "↺ Index": notes + knowledge, index only. Ignored while a job is running. */
export async function startIndexRebuild() {
  if (indexRebuildStore.getSnapshot().status === 'running') return;
  indexRebuildStore.set({ status: 'running', log: 'Starting…' });
  try {
    const res = (await api.reindexAll()) as StatusPayload;
    if (res?.error) {
      indexRebuildStore.set({ status: 'error', log: res.error });
      return;
    }
  } catch (err) {
    indexRebuildStore.set({
      status: 'error',
      log: err instanceof Error ? err.message : 'Request failed',
    });
    return;
  }
  ensurePolling();
}

/** Test hook. */
export function _resetIndexRebuild() {
  stopPolling();
  indexRebuildStore.set({ status: 'idle', log: '' });
}
