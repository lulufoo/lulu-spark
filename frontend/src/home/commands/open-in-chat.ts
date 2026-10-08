import * as api from '../../host/api.ts';
import { navigate } from '../../router/index.ts';
import { contextPercentFrom, contextUsageFrom, hydrateStaged, sessionIdOf, setHomeState } from '../state/store.ts';
import { requestComposerFocus } from './composer-focus.ts';

export type StageSourceInput = {
  kind: string;
  id: string;
};

export async function openPathInChat(path: string, source: StageSourceInput) {
  const abs = String(path || '').trim();
  if (!abs) throw new Error('Missing file path');
  const sourceKind = String(source?.kind || '').trim();
  const sourceId = String(source?.id || '').trim();
  if (!sourceId) throw new Error('Missing source');
  const created = (await api.invoke('create_chat_session')) as {
    session_id?: string;
    sessionId?: string;
  };
  const sessionId = sessionIdOf(created);
  if (!sessionId) throw new Error('No conversation');
  const result = (await api.invoke('stage_chat_document', {
    sessionId,
    path: abs,
    sourceKind,
    sourceId,
  })) as {
    staged?: unknown;
  };
  setHomeState((prev) => ({
    ...prev,
    currentSessionId: sessionId,
    contextPercent: contextPercentFrom(created),
    contextUsage: contextUsageFrom(created),
    staged: Object.hasOwn(result || {}, 'staged') ? hydrateStaged(result.staged) : prev.staged,
  }));
  requestComposerFocus();
  navigate('#/home');
}
