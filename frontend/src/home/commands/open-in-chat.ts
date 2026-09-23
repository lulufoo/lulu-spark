import * as api from '../../host/api.ts';
import { navigate } from '../../router/index.ts';
import { contextPercentFrom, hydrateStaged, sessionIdOf, setHomeState } from '../state/store.ts';
import { requestComposerFocus } from './composer-focus.ts';

export async function openPathInChat(path: string) {
  const abs = String(path || '').trim();
  if (!abs) throw new Error('Missing file path');
  const created = (await api.invoke('create_chat_session')) as {
    session_id?: string;
    sessionId?: string;
  };
  const sessionId = sessionIdOf(created);
  if (!sessionId) throw new Error('No conversation');
  const result = (await api.invoke('stage_chat_document', { sessionId, path: abs })) as {
    staged?: unknown;
  };
  setHomeState((prev) => ({
    ...prev,
    currentSessionId: sessionId,
    contextPercent: contextPercentFrom(created),
    staged: Object.hasOwn(result || {}, 'staged') ? hydrateStaged(result.staged) : prev.staged,
  }));
  requestComposerFocus();
  navigate('#/home');
}
