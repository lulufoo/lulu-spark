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
const DIAGNOSTIC_SCHEMA_VERSION = 1;

function createTurnTraceId() {
  const id = globalThis.crypto?.randomUUID?.();
  if (typeof id === 'string') return `ui_${id}`;
  return `ui_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 12)}`;
}

function logAssistantDiagnostic(traceId, event, elapsedMs, fields = {}) {
  const payload = {
    schema_version: DIAGNOSTIC_SCHEMA_VERSION,
    timestamp: new Date().toISOString(),
    component: 'assistant.ui',
    event,
    trace_id: traceId,
    elapsed_ms: Math.max(0, Math.round(elapsedMs)),
    fields,
  };
  console.info(`[assistant-diagnostic] ${JSON.stringify(payload)}`);
}

function persistAssistantTiming(invoke, traceId, phase, elapsedMs, inputToResponseMs) {
  void invoke('record_ai_assistant_timing', {
    traceId,
    phase,
    elapsedMs: Math.max(0, Math.round(elapsedMs)),
    inputToResponseMs,
  }).catch(() => {});
}

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
  let cancelling = false;
  let inputSubmittedAt = null;
  /** @type {{ role: 'user'|'assistant'|'notice', text: string, error?: boolean }[]} */
  let messages = [];
  let unlistenOpened = null;
  let unlistenBindingChanged = null;

  root.classList.add('ai-assistant-panel');
  root.innerHTML = `
    <div class="ai-assistant-unbound" data-role="unbound-content" role="status">
      <p class="ai-assistant-unbound-message">Unbound</p>
      <p class="ai-assistant-unbound-detail">No business context is currently bound.</p>
    </div>
    <div class="ai-assistant-bound-content" data-role="bound-content" hidden>
      <div class="ai-assistant-messages" data-role="messages" aria-live="polite"></div>
      <form class="ai-assistant-composer" data-role="form">
        <div class="ai-assistant-composer-shell">
          <textarea class="ai-assistant-input" data-role="input" rows="2" placeholder="Message… (Enter to send)" disabled></textarea>
          <div class="ai-assistant-composer-actions">
            <button type="button" class="ai-assistant-cancel" data-role="cancel" aria-label="Cancel" hidden disabled>Cancel</button>
            <button type="submit" class="ai-assistant-send" data-role="send" aria-label="Send" disabled>
              <svg class="ai-assistant-send-icon" viewBox="0 0 16 16" aria-hidden="true" focusable="false">
                <path fill="currentColor" d="M8 13.8a.75.75 0 0 1-.75-.75V4.86L5.03 7.08a.75.75 0 1 1-1.06-1.06l3.5-3.5a.75.75 0 0 1 1.06 0l3.5 3.5a.75.75 0 1 1-1.06 1.06L8.75 4.86v8.19a.75.75 0 0 1-.75.75z"/>
              </svg>
            </button>
          </div>
        </div>
      </form>
    </div>
  `;

  const unboundContent = root.querySelector('[data-role="unbound-content"]');
  const boundContent = root.querySelector('[data-role="bound-content"]');
  const messagesEl = root.querySelector('[data-role="messages"]');
  const form = root.querySelector('[data-role="form"]');
  const input = root.querySelector('[data-role="input"]');
  const cancelBtn = root.querySelector('[data-role="cancel"]');
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
    sendBtn.hidden = sending;
    cancelBtn.hidden = !sending;
    cancelBtn.disabled = !sending || cancelling;
    cancelBtn.textContent = cancelling ? 'Cancelling…' : 'Cancel';
  }

  function refreshComposerAndStatus() {
    unboundContent.hidden = hostBound;
    boundContent.hidden = !hostBound;
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

  async function sendMessage(text, inputStartedAt = performance.now()) {
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
    cancelling = false;
    setComposerEnabled(false);
    messages.push({ role: 'user', text });
    pushNotice('Working…');
    renderMessages();
    const traceId = createTurnTraceId();
    const invokeStartedAt = performance.now();
    logAssistantDiagnostic(traceId, 'turn.invoke.started', invokeStartedAt - inputStartedAt);
    try {
      const result = await invoke('agent_chat_turn', {
        sessionId,
        message: text,
        traceId,
      });
      const invokeCompletedAt = performance.now();
      const inputToResponseMs = Math.max(0, Math.round(invokeCompletedAt - inputStartedAt));
      logAssistantDiagnostic(traceId, 'turn.invoke.completed', invokeCompletedAt - invokeStartedAt, {
        outcome: 'ok',
        input_to_response_ms: inputToResponseMs,
      });
      persistAssistantTiming(
        invoke,
        traceId,
        'response_received',
        invokeCompletedAt - invokeStartedAt,
        inputToResponseMs,
      );
      messages = messages.filter(
        (m) =>
          !(
            m.role === 'notice' &&
            (m.text === 'Working…' || m.text === 'Cancelling…')
          ),
      );
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
      const renderStartedAt = performance.now();
      renderMessages();
      const renderElapsedMs = performance.now() - renderStartedAt;
      logAssistantDiagnostic(traceId, 'turn.render.completed', renderElapsedMs);
      persistAssistantTiming(invoke, traceId, 'render_completed', renderElapsedMs);
      refreshComposerAndStatus();
    } catch (err) {
      logAssistantDiagnostic(traceId, 'turn.invoke.completed', performance.now() - invokeStartedAt, {
        outcome: 'error',
      });
      messages = messages.filter(
        (m) =>
          !(
            m.role === 'notice' &&
            (m.text === 'Working…' || m.text === 'Cancelling…')
          ),
      );
      pushNotice(err?.message ? String(err.message) : 'Failed to send', true);
    } finally {
      sending = false;
      setComposerEnabled(composerShouldEnable());
    }
  }

  async function cancelMessage() {
    const invoke = getTauriInvoke();
    if (!invoke || !sending || cancelling) return;
    cancelling = true;
    messages = messages.map((m) =>
      m.role === 'notice' && m.text === 'Working…'
        ? { ...m, text: 'Cancelling…' }
        : m,
    );
    renderMessages();
    setComposerEnabled(false);
    try {
      const result = await invoke('cancel_ai_assistant_turn');
      if (result && typeof result === 'object' && result.ok === false) {
        throw new Error(String(result.error || 'Unable to cancel turn'));
      }
    } catch (err) {
      cancelling = false;
      messages = messages.map((m) =>
        m.role === 'notice' && m.text === 'Cancelling…'
          ? { ...m, text: 'Working…' }
          : m,
      );
      pushNotice(err?.message ? String(err.message) : 'Unable to cancel turn', true);
      setComposerEnabled(false);
    }
  }

  form.addEventListener('submit', (event) => {
    event.preventDefault();
    const text = String(input.value || '').trim();
    if (!text || sending) return;
    const inputStartedAt = inputSubmittedAt ?? performance.now();
    inputSubmittedAt = null;
    input.value = '';
    void sendMessage(text, inputStartedAt);
  });

  cancelBtn.addEventListener('click', () => {
    void cancelMessage();
  });

  // Enter sends; Shift+Enter inserts a newline (textarea default).
  input.addEventListener('keydown', (event) => {
    if (event.key !== 'Enter' || event.shiftKey || event.isComposing) return;
    event.preventDefault();
    if (input.disabled || sendBtn.disabled || sending) return;
    inputSubmittedAt = performance.now();
    if (typeof form.requestSubmit === 'function') {
      form.requestSubmit();
    } else {
      form.dispatchEvent(new Event('submit', { cancelable: true, bubbles: true }));
    }
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
    root.classList.remove('ai-assistant-panel');
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
