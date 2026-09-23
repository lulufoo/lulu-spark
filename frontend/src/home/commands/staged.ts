import { openFilePopup } from '../../file-popup/index.ts';
import * as api from '../../host/api.ts';
import { stagedIdentityKey } from '../state/identity.ts';
import {
  contextPercentFrom,
  contextUsageFrom,
  getHomeState,
  hydrateStaged,
  setHomeState,
  type HubStagedEntry,
} from '../state/store.ts';

export function openStagedFile(item: HubStagedEntry) {
  const path = String(item?.path || '').trim();
  if (!path) return;
  openFilePopup({
    path,
    title: item.title,
    identityKey: stagedIdentityKey(item.kind, path),
  });
}

export async function refreshStagedFromBinding(
  sid: string,
  gen: number,
  liveGen: () => number,
) {
  if (getHomeState().currentSessionId !== sid) return;
  try {
    const binding = (await api.invoke('get_ai_assistant_binding')) as Record<string, unknown>;
    if (liveGen() !== gen || getHomeState().currentSessionId !== sid) return;
    setHomeState((prev) => ({
      ...prev,
      staged: hydrateStaged(binding?.staged),
      contextPercent: contextPercentFrom(binding),
      contextUsage: contextUsageFrom(binding),
    }));
  } catch {
    /* turn already applied; keep prior staged */
  }
}

/** UI-only: remove one Stage F-handle from the current Chat. */
export async function unstageStaged(id: string) {
  const stagedId = String(id || '').trim();
  const sid = getHomeState().currentSessionId;
  if (!stagedId || !sid) return;
  try {
    const result = (await api.invoke('unstage_chat_staged', {
      sessionId: sid,
      id: stagedId,
    })) as { staged?: unknown };
    if (getHomeState().currentSessionId !== sid) return;
    setHomeState((prev) => ({ ...prev, staged: hydrateStaged(result?.staged) }));
  } catch (err) {
    const text =
      err instanceof Error && err.message
        ? err.message
        : typeof err === 'object' && err && 'message' in err
          ? String((err as { message?: unknown }).message || 'Failed to remove from Stage')
          : 'Failed to remove from Stage';
    setHomeState((prev) => ({
      ...prev,
      messages: [...prev.messages, { role: 'assistant', text, error: true }],
    }));
  }
}
