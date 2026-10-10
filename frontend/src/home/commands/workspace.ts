import { openFilePopup } from '../../file-popup/index.ts';
import * as api from '../../host/api.ts';
import { getHomeState, nowUnixSecs, setHomeState, type HubWorkspaceFile } from '../state/store.ts';

export type { HubWorkspaceFile };

export function hydrateWorkspace(raw: unknown): HubWorkspaceFile[] {
  if (!Array.isArray(raw)) return [];
  const out: HubWorkspaceFile[] = [];
  for (const row of raw) {
    if (!row || typeof row !== 'object') continue;
    const path = String((row as { path?: unknown }).path ?? '').trim();
    if (!path) continue;
    const title = String((row as { title?: unknown }).title ?? '').trim();
    out.push({
      path,
      title: title || path.split('/').pop() || path,
    });
  }
  return out;
}

export function openWorkspaceFile(item: HubWorkspaceFile) {
  const path = String(item?.path || '').trim();
  if (!path) return;
  openFilePopup({
    path,
    title: item.title,
  });
}

/** UI-only: delete one file from this Chat's SESSION_WORKSPACE_DIR. */
export async function deleteWorkspaceFile(path: string) {
  const abs = String(path || '').trim();
  const sid = getHomeState().currentSessionId;
  if (!abs || !sid) return;
  try {
    const result = (await api.invoke('delete_chat_workspace_file', {
      sessionId: sid,
      path: abs,
    })) as { files?: unknown };
    if (getHomeState().currentSessionId !== sid) return;
    setHomeState((prev) => ({ ...prev, workspace: hydrateWorkspace(result?.files) }));
  } catch (err) {
    const text =
      err instanceof Error && err.message
        ? err.message
        : typeof err === 'object' && err && 'message' in err
          ? String((err as { message?: unknown }).message || 'Failed to delete workspace file')
          : 'Failed to delete workspace file';
    setHomeState((prev) => ({
      ...prev,
      messages: [
        ...prev.messages,
        { role: 'assistant', text, error: true, createdAt: nowUnixSecs() },
      ],
    }));
  }
}

export async function refreshWorkspace(
  sid: string,
  gen: number,
  liveGen: () => number,
) {
  if (!sid || getHomeState().currentSessionId !== sid) return;
  try {
    const result = (await api.invoke('list_chat_workspace', {
      sessionId: sid,
    })) as { files?: unknown };
    if (liveGen() !== gen || getHomeState().currentSessionId !== sid) return;
    setHomeState((prev) => ({ ...prev, workspace: hydrateWorkspace(result?.files) }));
  } catch {
    /* turn already applied; keep prior workspace */
  }
}
