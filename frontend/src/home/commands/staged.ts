import { openFilePopup } from '../../file-popup/index.ts';
import * as api from '../../host/api.ts';
import { getHomeState, hydrateStaged, setHomeState } from '../state/store.ts';

export function openStagedFile(path: string, title: string) {
  openFilePopup({ path, title });
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
    setHomeState((prev) => ({ ...prev, staged: hydrateStaged(binding?.staged) }));
  } catch {
    /* turn already applied; keep prior staged */
  }
}
