import { createModuleStore } from '../../shared/module-store.ts';

export type ApprovalRequest = {
  requestId: string;
  type: string;
  path: string;
  sessionId: string;
};

export type ApprovalState = {
  queue: ApprovalRequest[];
};

export const approvalStore = createModuleStore<ApprovalState>({
  queue: [],
});

export function visibleApproval(
  queue: ApprovalRequest[],
  currentSessionId: string,
): ApprovalRequest | null {
  const sid = currentSessionId.trim();
  if (!sid) return null;
  return queue.find((item) => item.sessionId === sid) ?? null;
}

export function resetApprovalState() {
  approvalStore.set({ queue: [] });
}
