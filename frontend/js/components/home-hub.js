import * as api from '../api.js';
import { escHtml } from '../utils.js';

const BINDING_CHANGED_EVENT = 'ai-assistant:binding-changed';

function getTauriListen() {
  if (typeof window === 'undefined') return null;
  const listen = window.__TAURI__?.event?.listen;
  return typeof listen === 'function' ? listen : null;
}

const HOME_CHAT_LIST_LIMIT = 20;

function formatSessionWhen(updatedAt) {
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

function sessionListLabel(session) {
  const title = String(session?.title || '').trim();
  if (title) return title;
  return formatSessionWhen(session?.updated_at) || 'New conversation';
}

function hydrateTurns(turns) {
  if (!Array.isArray(turns)) return [];
  return turns
    .filter(
      (t) =>
        t &&
        typeof t === 'object' &&
        (t.role === 'user' || t.role === 'assistant') &&
        t.content != null &&
        String(t.content).length > 0,
    )
    .map((t) => ({
      role: t.role,
      text: String(t.content),
    }));
}

function sessionIdOf(payload) {
  if (!payload || typeof payload !== 'object') return '';
  const raw = payload.session_id ?? payload.sessionId;
  return raw == null ? '' : String(raw);
}

/**
 * @param {HTMLElement} container
 * @param {{ navigate?: (hash: string) => void, openReadLater?: () => void }} opts
 * @returns {() => void}
 */
export function mountHomeHub(container, { navigate, openReadLater } = {}) {
  container.innerHTML = `
    <div class="home-chat">
      <aside class="home-chat-sidebar">
        <nav class="home-chat-nav" aria-label="Workbench">
          <button type="button" class="home-chat-nav-item home-desktop-shortcut" data-home-entry="workbench">
            <span class="home-desktop-shortcut-icon" aria-hidden="true">📂</span>
            <span class="home-desktop-shortcut-label">Notes</span>
          </button>
          <button type="button" class="home-chat-nav-item home-desktop-shortcut" data-home-entry="read-later">
            <span class="home-desktop-shortcut-icon" aria-hidden="true">📑</span>
            <span class="home-desktop-shortcut-label">Read Later</span>
          </button>
          <button type="button" class="home-chat-nav-item home-desktop-shortcut" data-home-entry="corpus">
            <span class="home-desktop-shortcut-icon" aria-hidden="true">📚</span>
            <span class="home-desktop-shortcut-label">Knowledge</span>
          </button>
          <button type="button" class="home-chat-nav-item home-desktop-shortcut" data-home-entry="todo-tasks">
            <span class="home-desktop-shortcut-icon" aria-hidden="true">📋</span>
            <span class="home-desktop-shortcut-label">Todos</span>
          </button>
        </nav>
        <div class="home-chat-sessions-head">
          <span class="home-chat-sessions-title">Chats</span>
          <button type="button" class="home-chat-new" data-role="new-session" aria-label="New conversation" title="New conversation">
            <svg class="home-chat-new-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
              <path fill="currentColor" d="M11 5a1 1 0 1 1 2 0v6h6a1 1 0 1 1 0 2h-6v6a1 1 0 1 1-2 0v-6H5a1 1 0 1 1 0-2h6V5z"/>
            </svg>
          </button>
        </div>
        <div class="home-chat-sessions" data-role="session-list" role="list"></div>
      </aside>
      <section class="home-chat-main">
        <div class="home-chat-messages" data-role="messages" aria-live="polite"></div>
        <form class="home-chat-composer" data-role="form">
          <div class="home-chat-composer-dock">
            <textarea class="home-chat-input" data-role="input" rows="1" placeholder="Message…"></textarea>
            <button type="submit" class="home-chat-send" data-role="send" aria-label="Send" title="Send">
              <span class="home-chat-send-label">Send</span>
              <svg class="home-chat-send-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
                <path fill="currentColor" d="M5.2 11.1 18.6 4.4a.8.8 0 0 1 1.1.9l-3.7 13.5a.8.8 0 0 1-1.4.3l-3.6-4.7-4.8-1.6a.8.8 0 0 1 0-1.5z"/>
              </svg>
            </button>
          </div>
        </form>
      </section>
    </div>
  `;

  const root = container.querySelector('.home-chat');
  const listEl = container.querySelector('[data-role="session-list"]');
  const messagesEl = container.querySelector('[data-role="messages"]');
  const form = container.querySelector('[data-role="form"]');
  const input = container.querySelector('[data-role="input"]');
  const sendBtn = container.querySelector('[data-role="send"]');

  let sessions = [];
  let currentId = '';
  let messages = [];
  let hostBound = false;
  let sending = false;
  let fetchGen = 0;

  function renderSessions() {
    if (!sessions.length) {
      listEl.innerHTML = '<p class="home-chat-sessions-empty">No conversations yet.</p>';
      return;
    }
    listEl.innerHTML = sessions
      .map((s) => {
        const id = String(s.session_id || '');
        const title = sessionListLabel(s);
        const active = id && id === currentId ? ' is-active' : '';
        return `<button type="button" class="home-chat-session${active}" data-session-id="${escHtml(id)}" role="listitem">${escHtml(title)}</button>`;
      })
      .join('');
  }

  function renderMessages() {
    if (!hostBound) {
      messagesEl.innerHTML =
        '<div class="home-chat-thread"><p class="home-chat-empty">Chat requires a workspace Binding.</p></div>';
      return;
    }
    if (!currentId) {
      messagesEl.innerHTML =
        '<div class="home-chat-thread"><p class="home-chat-empty">Select a conversation or start a new one.</p></div>';
      return;
    }
    if (!messages.length) {
      messagesEl.innerHTML =
        '<div class="home-chat-thread"><p class="home-chat-empty">No messages yet.</p></div>';
      return;
    }
    const turns = messages
      .map((m) => {
        const kind = m.role === 'user' ? 'user' : m.error ? 'error' : 'assistant';
        return `<div class="home-chat-turn home-chat-turn--${kind}"><div class="home-chat-bubble home-chat-bubble--${kind}">${escHtml(m.text)}</div></div>`;
      })
      .join('');
    messagesEl.innerHTML = `<div class="home-chat-thread">${turns}</div>`;
    messagesEl.scrollTop = messagesEl.scrollHeight;
  }

  function syncComposerHeight() {
    input.style.height = '20px';
    const next = Math.min(Math.max(input.scrollHeight, 20), 120);
    input.style.height = `${next}px`;
    const composerH = form.offsetHeight;
    if (composerH > 0) messagesEl.style.paddingBottom = `${composerH}px`;
  }

  function applySessionPayload(payload, gen) {
    if (gen != null && gen !== fetchGen) return;
    if (!payload || typeof payload !== 'object') return;
    if (payload.session_id != null || payload.sessionId != null) {
      currentId = sessionIdOf(payload);
    }
    if (Object.hasOwn(payload, 'turns')) {
      messages = hydrateTurns(payload.turns);
    }
    renderSessions();
    renderMessages();
  }

  function setComposerEnabled(enabled) {
    input.disabled = !enabled;
    sendBtn.disabled = !enabled || sending;
  }

  async function refreshList() {
    const listed = await api.invoke('list_chat_sessions');
    sessions = Array.isArray(listed?.sessions)
      ? listed.sessions.slice(0, HOME_CHAT_LIST_LIMIT)
      : [];
    if (listed?.current_session_id) {
      currentId = String(listed.current_session_id);
    }
    renderSessions();
  }

  async function selectSession(sessionId) {
    const gen = ++fetchGen;
    currentId = String(sessionId || '');
    renderSessions();
    const payload = await api.invoke('select_chat_session', { sessionId });
    applySessionPayload(payload, gen);
    await refreshList();
    if (gen === fetchGen) renderMessages();
  }

  async function createSession() {
    const gen = ++fetchGen;
    const payload = await api.invoke('create_chat_session');
    applySessionPayload(payload, gen);
    await refreshList();
    if (gen === fetchGen) {
      renderMessages();
      input.focus();
    }
  }

  function showActionError(err) {
    messages.push({
      role: 'assistant',
      text: err?.message ? String(err.message) : 'Unable to start a conversation.',
      error: true,
    });
    renderMessages();
  }

  async function applyBindingState() {
    const gen = ++fetchGen;
    try {
      const summary = await api.invoke('query_binding');
      hostBound = summary && typeof summary === 'object' && summary.state === 'bound';
    } catch {
      hostBound = false;
    }
    if (!hostBound) {
      currentId = '';
      messages = [];
    }
    setComposerEnabled(hostBound);
    if (!hostBound) {
      renderSessions();
      renderMessages();
      return;
    }
    try {
      await refreshList();
      if (gen !== fetchGen) return;
      if (currentId) {
        const state = await api.invoke('get_ai_assistant_binding');
        applySessionPayload(state, gen);
      } else {
        renderMessages();
      }
    } catch {
      if (gen !== fetchGen) return;
      renderSessions();
      renderMessages();
    }
  }

  const onClick = (event) => {
    const entry = event.target.closest('[data-home-entry]');
    if (entry && root.contains(entry)) {
      const target = entry.dataset.homeEntry;
      if (target === 'workbench') navigate?.('#/workbench');
      else if (target === 'read-later') {
        if (typeof openReadLater === 'function') openReadLater();
        else navigate?.('#/read-later');
      } else if (target === 'corpus') navigate?.('#/corpus');
      else if (target === 'todo-tasks') navigate?.('#/todo-tasks');
      return;
    }
    const plus = event.target.closest('[data-role="new-session"]');
    if (plus && root.contains(plus)) {
      void createSession().catch(showActionError);
      return;
    }
    const item = event.target.closest('[data-session-id]');
    if (item && root.contains(item)) {
      const id = item.getAttribute('data-session-id');
      if (id) void selectSession(id).catch(showActionError);
    }
  };

  const onSubmit = (event) => {
    event.preventDefault();
    const text = String(input.value || '').trim();
    if (!text || sending) return;
    input.value = '';
    syncComposerHeight();
    void sendMessage(text);
  };

  const onKey = (event) => {
    if (event.key !== 'Enter' || event.shiftKey || event.isComposing) return;
    event.preventDefault();
    if (input.disabled || sendBtn.disabled || sending) return;
    if (typeof form.requestSubmit === 'function') form.requestSubmit();
    else form.dispatchEvent(new Event('submit', { cancelable: true, bubbles: true }));
  };

  async function sendMessage(text) {
    try {
      if (!hostBound) {
        messages.push({ role: 'assistant', text: 'Chat requires a workspace Binding.', error: true });
        renderMessages();
        return;
      }
      if (!currentId) {
        await createSession();
      }
      if (!currentId) {
        messages.push({ role: 'assistant', text: 'Unable to start a conversation.', error: true });
        renderMessages();
        return;
      }
      sending = true;
      setComposerEnabled(false);
      messages.push({ role: 'user', text });
      renderMessages();
      const result = await api.invoke('agent_chat_turn', {
        sessionId: currentId,
        message: text,
      });
      if (result?.busy) {
        messages.push({ role: 'assistant', text: String(result.reply_text || 'Busy — try again later') });
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
      renderMessages();
      await refreshList();
    } catch (err) {
      messages.push({
        role: 'assistant',
        text: err?.message ? String(err.message) : 'Failed to send',
        error: true,
      });
      renderMessages();
    } finally {
      sending = false;
      setComposerEnabled(hostBound);
    }
  }

  root.addEventListener('click', onClick);
  form.addEventListener('submit', onSubmit);
  input.addEventListener('keydown', onKey);
  input.addEventListener('input', syncComposerHeight);
  renderSessions();
  renderMessages();
  syncComposerHeight();
  void applyBindingState();

  let unlistenBinding = null;
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
    root.removeEventListener('click', onClick);
    form.removeEventListener('submit', onSubmit);
    input.removeEventListener('keydown', onKey);
    input.removeEventListener('input', syncComposerHeight);
    container.innerHTML = '';
  };
}
