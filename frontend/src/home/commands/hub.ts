import * as api from '../../host/api.ts';
import { refreshStagedFromBinding } from './staged.ts';
import {
  getHomeState,
  hydrateStaged,
  hydrateTurns,
  neighborSessionId,
  sessionIdOf,
  setHomeState,
  type HubSession,
} from '../state/store.ts';

export const BINDING_CHANGED_EVENT = 'ai-assistant:binding-changed';
export const HOME_CHAT_LIST_LIMIT = 20;

type TauriListen = (
  event: string,
  handler: (event: unknown) => void,
) => Promise<() => void>;

function getTauriListen(): TauriListen | null {
  if (typeof window === 'undefined') return null;
  const listen = (
    window as Window & {
      __TAURI__?: { event?: { listen?: TauriListen } };
    }
  ).__TAURI__?.event?.listen;
  return typeof listen === 'function' ? listen : null;
}

let fetchGen = 0;
let unlistenBinding: (() => void) | null = null;

export function resetHomeCommands() {
  fetchGen += 1;
  if (typeof unlistenBinding === 'function') unlistenBinding();
  unlistenBinding = null;
}

function applySessionPayload(payload: Record<string, unknown> | null, gen?: number) {
  if (gen != null && gen !== fetchGen) return;
  if (!payload || typeof payload !== 'object') return;
  setHomeState((prev) => ({
    ...prev,
    currentSessionId:
      payload.session_id != null || payload.sessionId != null
        ? sessionIdOf(payload)
        : prev.currentSessionId,
    messages: Object.hasOwn(payload, 'turns') ? hydrateTurns(payload.turns) : prev.messages,
    staged: Object.hasOwn(payload, 'staged') ? hydrateStaged(payload.staged) : prev.staged,
  }));
}

export async function refreshList(gen?: number) {
  const listed = (await api.invoke('list_chat_sessions')) as {
    sessions?: HubSession[];
    current_session_id?: string;
  };
  if (gen != null && gen !== fetchGen) return;
  setHomeState((prev) => ({
    ...prev,
    sessions: Array.isArray(listed?.sessions)
      ? listed.sessions.slice(0, HOME_CHAT_LIST_LIMIT)
      : [],
    currentSessionId: listed?.current_session_id
      ? String(listed.current_session_id)
      : prev.currentSessionId,
  }));
}

export async function selectSession(sessionId: string) {
  const gen = ++fetchGen;
  setHomeState((prev) => ({ ...prev, currentSessionId: String(sessionId || '') }));
  const payload = (await api.invoke('select_chat_session', { sessionId })) as Record<
    string,
    unknown
  >;
  applySessionPayload(payload, gen);
  await refreshList(gen);
}

export async function createSession() {
  const gen = ++fetchGen;
  const payload = (await api.invoke('create_chat_session')) as Record<string, unknown>;
  applySessionPayload(payload, gen);
  await refreshList(gen);
  return gen === fetchGen;
}

export async function deleteSession(sessionId: string) {
  const id = String(sessionId || '');
  const snap = getHomeState();
  if (!id || snap.inFlightIds.includes(id)) return;
  const nextId = neighborSessionId(snap.sessions, id);
  const wasCurrent = snap.currentSessionId === id;
  const gen = ++fetchGen;
  try {
    await api.invoke('delete_chat_session', { sessionId: id });
    if (gen !== fetchGen) return;
    await refreshList(gen);
    if (gen !== fetchGen || !wasCurrent) return;
    if (nextId) await selectSession(nextId);
    else setHomeState((prev) => ({ ...prev, currentSessionId: '', messages: [], staged: [] }));
  } catch (err) {
    showActionError(err instanceof Error ? err : { message: String(err) });
  }
}

export function showActionError(err: { message?: string } | undefined) {
  setHomeState((prev) => ({
    ...prev,
    messages: [
      ...prev.messages,
      {
        role: 'assistant',
        text: err?.message ? String(err.message) : 'Unable to start a conversation.',
        error: true,
      },
    ],
  }));
}

export async function applyBindingState() {
  const gen = ++fetchGen;
  console.info('[DEBUG-assistant] home: applyBindingState start', { gen });
  let hostBound = false;
  try {
    const summary = (await api.invoke('query_binding')) as { state?: string } | null;
    hostBound = Boolean(summary && typeof summary === 'object' && summary.state === 'bound');
    console.info('[DEBUG-assistant] home: query_binding', summary);
  } catch (err) {
    hostBound = false;
    console.info('[DEBUG-assistant] home: query_binding failed', err);
  }
  if (gen !== fetchGen) {
    console.info('[DEBUG-assistant] home: applyBindingState stale', { gen, fetchGen });
    return;
  }
  if (!hostBound) {
    // Discard the current session on Unbound (source-scan: currentSessionId = '').
    const currentSessionId = '';
    setHomeState((prev) => ({
      ...prev,
      hostBound: false,
      currentSessionId,
      messages: [],
      staged: [],
      progressByChat: Object.create(null) as Record<string, string>,
      inFlightIds: [],
    }));
    console.info('[DEBUG-assistant] home: unbound, skip list');
    return;
  }
  setHomeState((prev) => ({ ...prev, hostBound: true }));
  try {
    await refreshList(gen);
    if (gen !== fetchGen) return;
    console.info('[DEBUG-assistant] home: bound, list', {
      sessions: getHomeState().sessions.length,
      currentSessionId: getHomeState().currentSessionId,
    });
    if (getHomeState().currentSessionId) {
      const state = (await api.invoke('get_ai_assistant_binding')) as Record<string, unknown>;
      applySessionPayload(state, gen);
    }
  } catch (err) {
    console.info('[DEBUG-assistant] home: bound list/session failed', err);
  }
}

export async function sendMessage(text: string) {
  let sid = '';
  try {
    const snap = getHomeState();
    if (!snap.hostBound) {
      setHomeState((prev) => ({
        ...prev,
        messages: [
          ...prev.messages,
          { role: 'assistant', text: 'Chat requires a workspace Binding.', error: true },
        ],
      }));
      return;
    }
    if (!snap.currentSessionId) {
      await createSession();
    }
    if (!getHomeState().currentSessionId) {
      setHomeState((prev) => ({
        ...prev,
        messages: [
          ...prev.messages,
          { role: 'assistant', text: 'Unable to start a conversation.', error: true },
        ],
      }));
      return;
    }
    sid = getHomeState().currentSessionId;
    setHomeState((prev) => ({
      ...prev,
      inFlightIds: prev.inFlightIds.includes(sid) ? prev.inFlightIds : [...prev.inFlightIds, sid],
      messages: [...prev.messages, { role: 'user', text }],
    }));
    const channel = await api.createChannel((payload: { desc?: unknown } | string) => {
      const desc =
        payload && typeof payload === 'object'
          ? String(payload.desc ?? '')
          : String(payload ?? '');
      setHomeState((prev) => ({
        ...prev,
        progressByChat: { ...prev.progressByChat, [sid]: desc },
      }));
    });
    const result = (await api.invoke('agent_chat_turn', {
      sessionId: sid,
      message: text,
      progress: channel,
    })) as { busy?: boolean; reply_text?: string; terminal?: string };
    if (getHomeState().currentSessionId === sid) {
      if (result?.busy) {
        setHomeState((prev) => ({
          ...prev,
          messages: [
            ...prev.messages,
            {
              role: 'assistant',
              text: String(result.reply_text || 'Busy — try again later'),
            },
          ],
        }));
      } else {
        const reply = String(result?.reply_text || '');
        if (reply) {
          setHomeState((prev) => ({
            ...prev,
            messages: [
              ...prev.messages,
              {
                role: 'assistant',
                text: reply,
                error: result?.terminal === 'error',
              },
            ],
          }));
        }
      }
      await refreshStagedFromBinding(sid, fetchGen, () => fetchGen);
    }
    await refreshList(fetchGen);
  } catch (err) {
    if (getHomeState().currentSessionId) {
      setHomeState((prev) => ({
        ...prev,
        messages: [
          ...prev.messages,
          {
            role: 'assistant',
            text: err instanceof Error && err.message ? err.message : 'Failed to send',
            error: true,
          },
        ],
      }));
    }
  } finally {
    if (sid) {
      setHomeState((prev) => {
        const progressByChat = { ...prev.progressByChat };
        delete progressByChat[sid];
        return {
          ...prev,
          inFlightIds: prev.inFlightIds.filter((id) => id !== sid),
          progressByChat,
        };
      });
    }
  }
}

export function startHomeHub() {
  const listen = getTauriListen();
  console.info('[DEBUG-assistant] home: startHomeHub', {
    hasListen: Boolean(listen),
    hasTauri: Boolean(typeof window !== 'undefined' && window.__TAURI__),
  });
  void applyBindingState();
  if (!listen) {
    console.info('[DEBUG-assistant] home: binding-changed listen missing');
    return;
  }
  void listen(BINDING_CHANGED_EVENT, () => {
    console.info('[DEBUG-assistant] home: binding-changed received');
    void applyBindingState();
  }).then((fn) => {
    unlistenBinding = fn;
    console.info('[DEBUG-assistant] home: binding-changed listen attached');
  });
}

export function stopHomeHub() {
  if (typeof unlistenBinding === 'function') unlistenBinding();
  unlistenBinding = null;
}
