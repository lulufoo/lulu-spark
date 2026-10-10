import { openFilePopup } from '../../file-popup/index.ts';
import * as api from '../../host/api.ts';
import { getHomeState, setHomeState, type HubWorkspaceFile } from '../state/store.ts';

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
