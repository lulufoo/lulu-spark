/**
 * Plan-page AI assistant window shell.
 * Talks only via Host `agent_chat_turn` — never plan_task write commands.
 *
 * Host dual surface:
 * - Binding Contract: Set/Reset/query/execute (+ callbacks)
 * - Present: shell open/focus (maps to create_or_focus); Present does not imply Set.
 * Composer eligibility follows Binding Contract `query_binding` (state==bound).
 * Shell does not display or depend on todo master id / title.
 * 关壳 ≠ Reset — dispose notifies shell_close only; never Binding Contract Reset.
 */

import { escHtml } from './utils.js';

const OPENED_EVENT = 'ai-assistant:opened';
const BINDING_CHANGED_EVENT = 'ai-assistant:binding-changed';
/** Present surface name — not a Binding Contract op; Present≠bound. */
const PRESENT_SURFACE = 'Present';

function getTauriInvoke() {
  if (typeof window === 'undefined') return null;
  const invoke =
    window.__TAURI__?.core?.invoke || window.__TAURI_INTERNALS__?.invoke;
  return typeof invoke === 'function' ? invoke : null;
}

function getTauriListen() {
  if (typeof window === 'undefined') return null;
  const listen = window.__TAURI__?.event?.listen;
  return typeof listen === 'function' ? listen : null;
}

function bindingFromSearch() {
  if (typeof location === 'undefined') return null;
  const params = new URLSearchParams(location.search);
  const sessionId = params.get('session_id');
  if (!sessionId) return null;
  return {
    session_id: sessionId,
    busy: params.get('busy') === 'true',
    reply_text: params.get('reply_text') || '',
  };
}

/**
 * @param {HTMLElement} root
 * @param {{ autoBind?: boolean }} [opts]
 */
export function mountAiAssistant(root, opts = {}) {
  const { autoBind = true } = opts;
  let sessionId = '';
  /** Host Binding Contract state — composer gate. Present≠bound. */
  let hostBound = false;
  let hostBusy = false;
  let sending = false;
  /** @type {{ role: 'user'|'assistant'|'notice', text: string, error?: boolean }[]} */
  let messages = [];
  let unlistenOpened = null;
  let unlistenBindingChanged = null;

  root.innerHTML = `
    <header class="ai-assistant-header">
      <h1 class="ai-assistant-title">Assistant</h1>
      <p class="ai-assistant-bound" data-role="bound">Unbound</p>
    </header>
    <div class="ai-assistant-messages" data-role="messages" aria-live="polite"></div>
    <form class="ai-assistant-composer" data-role="form">
      <textarea class="ai-assistant-input" data-role="input" rows="2" placeholder="Message…" disabled></textarea>
      <button type="submit" class="ai-assistant-send" data-role="send" disabled>Send</button>
    </form>
  `;

  const boundEl = root.querySelector('[data-role="bound"]');
  const messagesEl = root.querySelector('[data-role="messages"]');
  const form = root.querySelector('[data-role="form"]');
  const input = root.querySelector('[data-role="input"]');
  const sendBtn = root.querySelector('[data-role="send"]');

  function renderMessages() {
    messagesEl.innerHTML = messages
      .map((m) => {
        const kind =
          m.role === 'user'
            ? 'user'
            : m.role === 'notice'
              ? 'notice'
              : 'assistant';
        const errClass = m.error ? ' ai-assistant-bubble--error' : '';
        return `<div class="ai-assistant-bubble ai-assistant-bubble--${kind}${errClass}">${escHtml(m.text)}</div>`;
      })
      .join('');
    messagesEl.scrollTop = messagesEl.scrollHeight;
  }

  /**
   * Hydrate chat bubbles from Host binding.turns (mount / re-show).
   * Does not Reset Binding or switch live session.
   * @param {unknown} turns
   */
  function hydrateTurns(turns) {
    if (!Array.isArray(turns)) return;
    messages = turns
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
    renderMessages();
  }

  function composerShouldEnable() {
    // Binding Contract gate: Present alone does not imply Set / does not enable chat.
    return hostBound && !hostBusy && !sending;
  }

  function setComposerEnabled(enabled) {
    input.disabled = !enabled;
    sendBtn.disabled = !enabled || sending;
  }

  function refreshComposerAndStatus() {
    boundEl.textContent = hostBound ? 'Bound' : 'Unbound';
    setComposerEnabled(composerShouldEnable());
  }

  /**
   * Apply chat-session UX payload. Does not set Binding Contract gate.
   * @param {object} payload
   */
  function applySessionPayload(payload) {
    if (!payload || typeof payload !== 'object') return;
    if (payload.session_id != null) {
      sessionId = String(payload.session_id || '');
    }
    if (typeof payload.busy === 'boolean') {
      hostBusy = payload.busy;
    }
    if (payload.busy && payload.reply_text) {
      messages = [];
      messages.push({ role: 'notice', text: String(payload.reply_text) });
      renderMessages();
    } else if (Array.isArray(payload.turns)) {
      hydrateTurns(payload.turns);
    }
    refreshComposerAndStatus();
  }

  /** @param {'bound'|'unbound'|string} state */
  function applyHostContractState(state) {
    hostBound = state === 'bound';
    // Discard cached sessionId whenever Host binding state changes (composer follows query_binding).
    sessionId = '';
    // Reset/Unbound: clear in-memory bubbles (Host turns will be empty on next pull).
    if (!hostBound) {
      messages = [];
      renderMessages();
    }
    refreshComposerAndStatus();
  }

  /** @deprecated Use applySessionPayload + applyHostContractState; kept for tests/callers. */
  function applyBinding(payload) {
    applySessionPayload(payload);
  }

  function pushNotice(text, error = false) {
    messages.push({ role: 'notice', text, error });
    renderMessages();
  }

  async function ensureSessionId(invoke) {
    if (sessionId) return true;
    try {
      const state = await invoke('get_ai_assistant_binding');
      if (state && typeof state === 'object' && state.session_id) {
        applySessionPayload(state);
      }
    } catch {
      // fall through
    }
    if (sessionId) return true;
    try {
      const ensured = await invoke('ensure_ai_assistant_session');
      if (ensured && typeof ensured === 'object' && ensured.session_id) {
        applySessionPayload(ensured);
      }
    } catch {
      // fall through
    }
    return Boolean(sessionId);
  }

  async function sendMessage(text) {
    const invoke = getTauriInvoke();
    if (!invoke) {
      pushNotice('Tauri invoke unavailable', true);
      return;
    }
    if (!hostBound) {
      pushNotice('Unbound — chat requires a Binding Contract Set', true);
      return;
    }
    if (!(await ensureSessionId(invoke))) {
      pushNotice('Chat session unavailable', true);
      return;
    }
    sending = true;
    setComposerEnabled(false);
    messages.push({ role: 'user', text });
    pushNotice('Working…');
    renderMessages();
    try {
      const result = await invoke('agent_chat_turn', {
        sessionId,
        message: text,
      });
      messages = messages.filter((m) => !(m.role === 'notice' && m.text === 'Working…'));
      if (result && typeof result === 'object') {
        if (result.busy) {
          pushNotice(String(result.reply_text || 'Busy — try again later'));
        } else {
          const reply = String(result.reply_text || '');
          if (reply) {
            messages.push({
              role: 'assistant',
              text: reply,
              error: result.terminal === 'error',
            });
          }
        }
      }
      renderMessages();
      refreshComposerAndStatus();
    } catch (err) {
      messages = messages.filter((m) => !(m.role === 'notice' && m.text === 'Working…'));
      pushNotice(err?.message ? String(err.message) : 'Failed to send', true);
    } finally {
      sending = false;
      setComposerEnabled(composerShouldEnable());
    }
  }

  form.addEventListener('submit', (event) => {
    event.preventDefault();
    const text = String(input.value || '').trim();
    if (!text || sending) return;
    input.value = '';
    void sendMessage(text);
  });

  async function bindOpenedListener() {
    const listen = getTauriListen();
    if (!listen) return;
    unlistenOpened = await listen(OPENED_EVENT, (event) => {
      applySessionPayload(event?.payload);
    });
  }

  async function bindBindingChangedListener() {
    const listen = getTauriListen();
    if (!listen) return;
    unlistenBindingChanged = await listen(BINDING_CHANGED_EVENT, (event) => {
      const state = event?.payload?.state;
      if (typeof state === 'string') {
        applyHostContractState(state);
      }
    });
  }

  /** Sync Binding Contract gate from Host query (Present≠bound). */
  async function pullHostContractState() {
    const invoke = getTauriInvoke();
    if (!invoke) return;
    try {
      const summary = await invoke('query_binding');
      if (summary && typeof summary === 'object') {
        applyHostContractState(String(summary.state || 'unbound'));
      }
    } catch {
      // Non-fatal; remain unbound until a later sync.
    }
  }

  /** Heal first-open race: Host may emit before this window's listener is ready. */
  async function pullSessionFromHost() {
    const invoke = getTauriInvoke();
    if (!invoke) return;
    try {
      const state = await invoke('get_ai_assistant_binding');
      if (state && typeof state === 'object' && state.session_id) {
        applySessionPayload(state);
      }
    } catch {
      // Non-fatal; event path or later open may still bind.
    }
  }

  if (autoBind) {
    const fromUrl = bindingFromSearch();
    if (fromUrl) applySessionPayload(fromUrl);
    void (async () => {
      await bindOpenedListener();
      await bindBindingChangedListener();
      await pullHostContractState();
      await pullSessionFromHost();
    })();
  }

  function dispose() {
    if (typeof unlistenOpened === 'function') {
      void unlistenOpened();
    }
    if (typeof unlistenBindingChanged === 'function') {
      void unlistenBindingChanged();
    }
    const invoke = getTauriInvoke();
    if (invoke) {
      void invoke('shell_close_ai_assistant').catch(() => {});
    }
    root.innerHTML = '';
  }

  return {
    dispose,
    applyBinding,
    applyHostContractState,
    getState: () => ({
      sessionId,
      hostBound,
      messages,
      // Shell UX session ≠ Binding Contract bound (Present does not imply Set).
      presentSurface: PRESENT_SURFACE,
    }),
  };
}

/**
 * AI Assistant content adapter for the home-entry shell content slot.
 * Shell owns overlay chrome; this module reuses mountAiAssistant for chat UI.
 * Must not Reset Binding or switch live session.
 * @returns {{ mount: (slotEl: HTMLElement, ctx?: { host?: unknown }) => { unmount: () => void } }}
 */
export function createAiAssistantContentAdapter() {
  return {
    /**
     * @param {HTMLElement} slotEl
     * @param {{ host?: unknown }} [_ctx]
     */
    mount(slotEl, _ctx = {}) {
      const panel = mountAiAssistant(slotEl, { autoBind: true });
      return {
        unmount() {
          panel.dispose();
        },
      };
    },
  };
}

const bootstrapRoot = document.getElementById('ai-assistant-root');
if (bootstrapRoot) {
  mountAiAssistant(bootstrapRoot);
}
