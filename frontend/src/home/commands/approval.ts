import * as api from '../../host/api.ts';
import { getHomeState } from '../state/store.ts';
import {
  approvalStore,
  resetApprovalState,
  visibleApproval,
  type ApprovalRequest,
} from '../state/approval.ts';

export const APPROVAL_REQUESTED_EVENT = 'ai-assistant:approval-requested';

export type ApprovalChoice = 'allow_once' | 'allow_session' | 'deny';

export type { ApprovalRequest };

export function parseApprovalPayload(payload: unknown): ApprovalRequest | null {
  if (!payload || typeof payload !== 'object') return null;
  const rec = payload as Record<string, unknown>;
  const requestId = String(rec.request_id ?? '').trim();
  const type = String(rec.type ?? '').trim();
  const path = String(rec.path ?? '').trim();
  const sessionId = String(rec.session_id ?? '').trim();
  if (!requestId || !path || !sessionId) return null;
  return { requestId, type, path, sessionId };
}

export function enqueueApproval(payload: unknown) {
  const req = parseApprovalPayload(payload);
  if (!req) return;
  approvalStore.set((prev) => {
    if (prev.queue.some((item) => item.requestId === req.requestId)) return prev;
    return { queue: [...prev.queue, req] };
  });
}

export function pendingApproval(): ApprovalRequest | null {
  return visibleApproval(approvalStore.getSnapshot().queue, getHomeState().currentSessionId);
}

export function startApprovalHub() {
  const listen =
    typeof window !== 'undefined' && window.__TAURI__?.event?.listen;
  if (typeof listen !== 'function') return;
  void listen(APPROVAL_REQUESTED_EVENT, (event: { payload?: unknown }) => {
    enqueueApproval(event?.payload);
  });
}

export async function resolveVisibleApproval(choice: ApprovalChoice) {
  const req = pendingApproval();
  if (!req) return;
  await api.invoke('resolve_approval', {
    requestId: req.requestId,
    choice,
  });
  dropRequest(req.requestId);
}

export async function dismissVisibleApproval() {
  await resolveVisibleApproval('deny');
}

export function resetApprovalForTests() {
  resetApprovalState();
}

function dropRequest(requestId: string) {
  approvalStore.set((prev) => ({
    queue: prev.queue.filter((item) => item.requestId !== requestId),
  }));
}
