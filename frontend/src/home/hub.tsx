import { flushSync } from 'react-dom';
import { createRoot, type Root } from 'react-dom/client';
import * as api from '../host/api.ts';
import { hydrateHomeChatMarkdown, renderHomeChatMarkdown } from './chat-render.ts';

const BINDING_CHANGED_EVENT = 'ai-assistant:binding-changed';
const HOME_CHAT_LIST_LIMIT = 20;

type HubSession = {
  session_id?: string;
  title?: string;
  updated_at?: number;
};

type HubMessage = {
  role: string;
  text: string;
  error?: boolean;
};

function getTauriListen() {
  if (typeof window === 'undefined') return null;
  const listen = window.__TAURI__?.event?.listen;
  return typeof listen === 'function' ? listen : null;
}

function formatSessionWhen(updatedAt: unknown) {
  const ts = Number(updatedAt);
  if (!Number.isFinite(ts) || ts <= 0) return '';
  const date = new Date(ts * 1000);
  if (Number.isNaN(date.getTime())) return '';
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  const hour = String(date.getHours()).padStart(2, '0');
  const minute = String(date.getMinutes()).padStart(2, '0');
  return `${year}-${month}-${day} ${hour}:${minute}`;
}

function sessionListLabel(session: HubSession) {
  const title = String(session?.title || '').trim();
  if (title) return title;
  return formatSessionWhen(session?.updated_at) || 'New conversation';
}

function hydrateTurns(turns: unknown): HubMessage[] {
  if (!Array.isArray(turns)) return [];
  return turns
    .filter(
      (t) =>
        t &&
        typeof t === 'object' &&
        ((t as HubMessage).role === 'user' || (t as HubMessage).role === 'assistant') &&
        (t as { content?: unknown }).content != null &&
        String((t as { content?: unknown }).content).length > 0,
    )
    .map((t) => ({
      role: (t as HubMessage).role,
      text: String((t as { content: unknown }).content),
    }));
}

function sessionIdOf(payload: { session_id?: unknown; sessionId?: unknown } | null) {
  if (!payload || typeof payload !== 'object') return '';
  const raw = payload.session_id ?? payload.sessionId;
  return raw == null ? '' : String(raw);
}

function SessionList({
  sessions,
  currentSessionId,
  progressByChat,
}: {
  sessions: HubSession[];
  currentSessionId: string;
  progressByChat: Record<string, string>;
}) {
  if (!sessions.length) {
    return <p className="home-chat-sessions-empty">No conversations yet.</p>;
  }
  return (
    <>
      {sessions.map((s) => {
        const id = String(s.session_id || '');
        const title = sessionListLabel(s);
        const active = id && id === currentSessionId ? ' is-active' : '';
        const flying = Boolean(progressByChat[id]);
        return (
          <button
            key={id || title}
            type="button"
            className={`home-chat-session${active}`}
            data-session-id={id}
            role="listitem"
          >
            {title}
            {flying ? (
              <span className="home-chat-session-progress" aria-hidden="true">
                …
              </span>
            ) : null}
          </button>
        );
      })}
    </>
  );
}

function MessageThread({
  hostBound,
  currentSessionId,
  messages,
}: {
  hostBound: boolean;
  currentSessionId: string;
  messages: HubMessage[];
}) {
  if (!hostBound) {
    return (
      <div className="home-chat-thread">
        <p className="home-chat-empty">Chat requires a workspace Binding.</p>
      </div>
    );
  }
  if (!currentSessionId) {
    return (
      <div className="home-chat-thread">
        <p className="home-chat-empty">Select a conversation or start a new one.</p>
      </div>
    );
  }
  if (!messages.length) {
    return (
      <div className="home-chat-thread">
        <p className="home-chat-empty">No messages yet.</p>
      </div>
    );
  }
  return (
    <div className="home-chat-thread">
      {messages.map((m, i) => {
        const kind = m.role === 'user' ? 'user' : m.error ? 'error' : 'assistant';
        return (
          <div key={i} className={`home-chat-turn home-chat-turn--${kind}`}>
            <div className={`home-chat-bubble home-chat-bubble--${kind}`}>
              {kind === 'assistant' ? (
                <div
                  className="home-chat-md"
                  dangerouslySetInnerHTML={{ __html: renderHomeChatMarkdown(m.text) }}
                />
              ) : (
                m.text
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}

function HomeHubView({
  sessions,
  currentSessionId,
  messages,
  hostBound,
  progressByChat,
  composerLocked,
  progressHint,
}: {
  sessions: HubSession[];
  currentSessionId: string;
  messages: HubMessage[];
  hostBound: boolean;
  progressByChat: Record<string, string>;
  composerLocked: boolean;
  progressHint: string;
}) {
  return (
    <div className="home-chat">
      <aside className="home-chat-sidebar">
        <nav className="home-chat-nav" aria-label="Workbench">
          <button type="button" className="home-chat-nav-item home-desktop-shortcut" data-home-entry="workbench">
            <span className="home-desktop-shortcut-icon" aria-hidden="true">
              📂
            </span>
            <span className="home-desktop-shortcut-label">Notes</span>
          </button>
          <button type="button" className="home-chat-nav-item home-desktop-shortcut" data-home-entry="read-later">
            <span className="home-desktop-shortcut-icon" aria-hidden="true">
              📑
            </span>
            <span className="home-desktop-shortcut-label">Read Later</span>
          </button>
          <button type="button" className="home-chat-nav-item home-desktop-shortcut" data-home-entry="corpus">
            <span className="home-desktop-shortcut-icon" aria-hidden="true">
              📚
            </span>
            <span className="home-desktop-shortcut-label">Knowledge</span>
          </button>
          <button type="button" className="home-chat-nav-item home-desktop-shortcut" data-home-entry="todo-tasks">
            <span className="home-desktop-shortcut-icon" aria-hidden="true">
              📋
            </span>
            <span className="home-desktop-shortcut-label">Todos</span>
          </button>
        </nav>
        <div className="home-chat-sessions-head">
          <span className="home-chat-sessions-title">Chats</span>
          <button
            type="button"
            className="home-chat-new"
            data-role="new-session"
            aria-label="New conversation"
            title="New conversation"
          >
            <svg className="home-chat-new-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
              <path
                fill="currentColor"
                d="M11 5a1 1 0 1 1 2 0v6h6a1 1 0 1 1 0 2h-6v6a1 1 0 1 1-2 0v-6H5a1 1 0 1 1 0-2h6V5z"
              />
            </svg>
          </button>
        </div>
        <div className="home-chat-sessions" data-role="session-list" role="list">
          <SessionList
            sessions={sessions}
            currentSessionId={currentSessionId}
            progressByChat={progressByChat}
          />
        </div>
      </aside>
      <section className="home-chat-main">
        <div className="home-chat-messages" data-role="messages" aria-live="polite">
          <MessageThread
            hostBound={hostBound}
            currentSessionId={currentSessionId}
            messages={messages}
          />
        </div>
        <form className="home-chat-composer" data-role="form">
          <p className="home-chat-progress" data-role="progress-hint" hidden={!progressHint}>
            {progressHint}
          </p>
          <div className="home-chat-composer-dock">
            <textarea
              className="home-chat-input"
              data-role="input"
              rows={1}
              placeholder="Message…"
              disabled={composerLocked}
            />
            <button
              type="submit"
              className="home-chat-send"
              data-role="send"
              aria-label="Send"
              title="Send"
              disabled={composerLocked}
            >
              <span className="home-chat-send-label">Send</span>
              <svg className="home-chat-send-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
                <path
                  fill="currentColor"
                  d="M5.2 11.1 18.6 4.4a.8.8 0 0 1 1.1.9l-3.7 13.5a.8.8 0 0 1-1.4.3l-3.6-4.7-4.8-1.6a.8.8 0 0 1 0-1.5z"
                />
              </svg>
            </button>
          </div>
        </form>
      </section>
    </div>
  );
}

/**
 * @param {HTMLElement} container
 * @param {{ navigate?: (hash: string) => void, openReadLater?: () => void }} opts
 * @returns {() => void}
 */
export function mountHomeHub(
  container: HTMLElement,
  { navigate, openReadLater }: { navigate?: (hash: string) => void; openReadLater?: () => void } = {},
) {
  let sessions: HubSession[] = [];
  let currentSessionId = '';
  let messages: HubMessage[] = [];
  let hostBound = false;
  const progressByChat: Record<string, string> = Object.create(null);
  const inFlight = new Set<string>();
  let fetchGen = 0;
  let mdPaintGen = 0;
  let lastMessagePaintKey = '';
  let root: Root | null = createRoot(container);

  function messagePaintKey() {
    return [
      hostBound ? '1' : '0',
      currentSessionId,
      ...messages.map((m) => `${m.role}\0${m.text}\0${m.error ? '1' : '0'}`),
    ].join('\n');
  }

  function composerLocked() {
    return !hostBound || inFlight.has(currentSessionId);
  }

  function progressHint() {
    return currentSessionId ? String(progressByChat[currentSessionId] || '') : '';
  }

  function paint() {
    if (!root) return;
    const nextKey = messagePaintKey();
    const messagesChanged = nextKey !== lastMessagePaintKey;
    lastMessagePaintKey = nextKey;
    flushSync(() => {
      root!.render(
        <HomeHubView
          sessions={sessions}
          currentSessionId={currentSessionId}
          messages={messages}
          hostBound={hostBound}
          progressByChat={progressByChat}
          composerLocked={composerLocked()}
          progressHint={progressHint()}
        />,
      );
    });
    syncComposerHeight();
    const messagesEl = container.querySelector('[data-role="messages"]');
    if (!(messagesEl instanceof HTMLElement)) return;
    const needsHydrate =
      messagesChanged || Boolean(messagesEl.querySelector('pre > code.language-mermaid'));
    if (!messagesChanged && !needsHydrate) return;
    if (messagesChanged) {
      messagesEl.scrollTop = messagesEl.scrollHeight;
    }
    if (!needsHydrate) return;
    const paintGen = ++mdPaintGen;
    void hydrateHomeChatMarkdown(messagesEl).then(() => {
      if (paintGen === mdPaintGen && messagesChanged) {
        messagesEl.scrollTop = messagesEl.scrollHeight;
      }
    });
  }

  function applySessionPayload(payload: Record<string, unknown> | null, gen?: number) {
    if (gen != null && gen !== fetchGen) return;
    if (!payload || typeof payload !== 'object') return;
    if (payload.session_id != null || payload.sessionId != null) {
      currentSessionId = sessionIdOf(payload);
    }
    if (Object.hasOwn(payload, 'turns')) {
      messages = hydrateTurns(payload.turns);
    }
    paint();
  }

  function syncComposerHeight() {
    const input = container.querySelector('[data-role="input"]');
    const form = container.querySelector('[data-role="form"]');
    const messagesEl = container.querySelector('[data-role="messages"]');
    if (!(input instanceof HTMLTextAreaElement) || !(form instanceof HTMLElement)) return;
    input.style.height = '20px';
    const next = Math.min(Math.max(input.scrollHeight, 20), 120);
    input.style.height = `${next}px`;
    const composerH = form.offsetHeight;
    if (composerH > 0 && messagesEl instanceof HTMLElement) {
      messagesEl.style.paddingBottom = `${composerH}px`;
    }
  }

  async function refreshList() {
    const listed = (await api.invoke('list_chat_sessions')) as {
      sessions?: HubSession[];
      current_session_id?: string;
    };
    sessions = Array.isArray(listed?.sessions)
      ? listed.sessions.slice(0, HOME_CHAT_LIST_LIMIT)
      : [];
    if (listed?.current_session_id) {
      currentSessionId = String(listed.current_session_id);
    }
    paint();
  }

  async function selectSession(sessionId: string) {
    const gen = ++fetchGen;
    currentSessionId = String(sessionId || '');
    paint();
    const payload = (await api.invoke('select_chat_session', { sessionId })) as Record<
      string,
      unknown
    >;
    applySessionPayload(payload, gen);
    await refreshList();
    if (gen === fetchGen) paint();
  }

  async function createSession() {
    const gen = ++fetchGen;
    const payload = (await api.invoke('create_chat_session')) as Record<string, unknown>;
    applySessionPayload(payload, gen);
    await refreshList();
    if (gen === fetchGen) {
      paint();
      const input = container.querySelector('[data-role="input"]');
      if (input instanceof HTMLTextAreaElement) input.focus();
    }
  }

  function showActionError(err: { message?: string } | undefined) {
    messages.push({
      role: 'assistant',
      text: err?.message ? String(err.message) : 'Unable to start a conversation.',
      error: true,
    });
    paint();
  }

  async function applyBindingState() {
    const gen = ++fetchGen;
    try {
      const summary = (await api.invoke('query_binding')) as { state?: string } | null;
      hostBound = Boolean(summary && typeof summary === 'object' && summary.state === 'bound');
    } catch {
      hostBound = false;
    }
    if (!hostBound) {
      currentSessionId = '';
      messages = [];
      Object.keys(progressByChat).forEach((key) => {
        delete progressByChat[key];
      });
      inFlight.clear();
    }
    if (!hostBound) {
      paint();
      return;
    }
    try {
      await refreshList();
      if (gen !== fetchGen) return;
      if (currentSessionId) {
        const state = (await api.invoke('get_ai_assistant_binding')) as Record<string, unknown>;
        applySessionPayload(state, gen);
      } else {
        paint();
      }
    } catch {
      if (gen !== fetchGen) return;
      paint();
    }
  }

  const onClick = (event: Event) => {
    const target = event.target as HTMLElement;
    const chatRoot = container.querySelector('.home-chat');
    const entry = target.closest('[data-home-entry]');
    if (entry && chatRoot?.contains(entry)) {
      const dest = (entry as HTMLElement).dataset.homeEntry;
      if (dest === 'workbench') navigate?.('#/workbench');
      else if (dest === 'read-later') {
        if (typeof openReadLater === 'function') openReadLater();
        else navigate?.('#/read-later');
      } else if (dest === 'corpus') navigate?.('#/corpus');
      else if (dest === 'todo-tasks') navigate?.('#/todo-tasks');
      return;
    }
    const plus = target.closest('[data-role="new-session"]');
    if (plus && chatRoot?.contains(plus)) {
      void createSession().catch(showActionError);
      return;
    }
    const item = target.closest('[data-session-id]');
    if (item && chatRoot?.contains(item)) {
      const id = item.getAttribute('data-session-id');
      if (id) void selectSession(id).catch(showActionError);
    }
  };

  const onSubmit = (event: Event) => {
    event.preventDefault();
    const input = container.querySelector('[data-role="input"]');
    const text =
      input instanceof HTMLTextAreaElement ? String(input.value || '').trim() : '';
    if (!text || inFlight.has(currentSessionId)) return;
    if (input instanceof HTMLTextAreaElement) input.value = '';
    syncComposerHeight();
    void sendMessage(text);
  };

  const onKey = (event: KeyboardEvent) => {
    if (event.key !== 'Enter' || event.shiftKey || event.isComposing) return;
    event.preventDefault();
    const input = container.querySelector('[data-role="input"]');
    const sendBtn = container.querySelector('[data-role="send"]');
    const form = container.querySelector('[data-role="form"]');
    if (
      (input instanceof HTMLTextAreaElement && input.disabled) ||
      (sendBtn instanceof HTMLButtonElement && sendBtn.disabled) ||
      inFlight.has(currentSessionId)
    ) {
      return;
    }
    if (form instanceof HTMLFormElement) {
      if (typeof form.requestSubmit === 'function') form.requestSubmit();
      else form.dispatchEvent(new Event('submit', { cancelable: true, bubbles: true }));
    }
  };

  async function sendMessage(text: string) {
    let sid = '';
    try {
      if (!hostBound) {
        messages.push({
          role: 'assistant',
          text: 'Chat requires a workspace Binding.',
          error: true,
        });
        paint();
        return;
      }
      if (!currentSessionId) {
        await createSession();
      }
      if (!currentSessionId) {
        messages.push({
          role: 'assistant',
          text: 'Unable to start a conversation.',
          error: true,
        });
        paint();
        return;
      }
      sid = currentSessionId;
      inFlight.add(sid);
      messages.push({ role: 'user', text });
      paint();
      const channel = await api.createChannel((payload: { desc?: unknown } | string) => {
        const desc =
          payload && typeof payload === 'object'
            ? String(payload.desc ?? '')
            : String(payload ?? '');
        progressByChat[sid] = desc;
        paint();
      });
      const result = (await api.invoke('agent_chat_turn', {
        sessionId: sid,
        message: text,
        progress: channel,
      })) as { busy?: boolean; reply_text?: string; terminal?: string };
      if (currentSessionId === sid) {
        if (result?.busy) {
          messages.push({
            role: 'assistant',
            text: String(result.reply_text || 'Busy — try again later'),
          });
        } else {
          const reply = String(result?.reply_text || '');
          if (reply) {
            messages.push({
              role: 'assistant',
              text: reply,
              error: result?.terminal === 'error',
            });
          }
        }
        paint();
      }
      await refreshList();
    } catch (err) {
      if (currentSessionId) {
        messages.push({
          role: 'assistant',
          text: err instanceof Error && err.message ? err.message : 'Failed to send',
          error: true,
        });
        paint();
      }
    } finally {
      if (sid) {
        inFlight.delete(sid);
        delete progressByChat[sid];
        paint();
      }
    }
  }

  paint();
  const chatRoot = container.querySelector('.home-chat');
  const form = container.querySelector('[data-role="form"]');
  const input = container.querySelector('[data-role="input"]') as HTMLTextAreaElement | null;
  chatRoot?.addEventListener('click', onClick);
  form?.addEventListener('submit', onSubmit);
  input?.addEventListener('keydown', onKey);
  input?.addEventListener('input', syncComposerHeight);
  void applyBindingState();

  let unlistenBinding: (() => void) | null = null;
  const listen = getTauriListen();
  if (listen) {
    void listen(BINDING_CHANGED_EVENT, () => {
      void applyBindingState();
    }).then((fn) => {
      unlistenBinding = fn;
    });
  }

  return () => {
    if (typeof unlistenBinding === 'function') unlistenBinding();
    chatRoot?.removeEventListener('click', onClick);
    form?.removeEventListener('submit', onSubmit);
    input?.removeEventListener('keydown', onKey);
    input?.removeEventListener('input', syncComposerHeight);
    flushSync(() => {
      root?.unmount();
    });
    root = null;
    container.innerHTML = '';
  };
}
