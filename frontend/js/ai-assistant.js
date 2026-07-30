/**
 * Plan-page AI assistant window shell.
 * Talks only via Host `agent_chat_turn` — never plan_task write commands.
 *
 * Host dual surface:
 * - Binding Contract: Set/Reset/query/execute (+ callbacks)
 * - Present: shell open/focus (maps to create_or_focus); Present does not imply Set.
 * 关壳 ≠ Reset — dispose notifies shell_close only; never Binding Contract Reset.
 */

import { escHtml } from './utils.js';

const OPENED_EVENT = 'ai-assistant:opened';
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
    bound_master_task_id: params.get('bound_master_task_id') || '',
    bound_title: params.get('bound_title') || '',
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
  let boundMasterTaskId = '';
  let boundTitle = '';
  let sending = false;
  /** @type {{ role: 'user'|'assistant'|'notice', text: string, error?: boolean }[]} */
  let messages = [];
  let unlistenOpened = null;

  root.innerHTML = `
    <header class="ai-assistant-header">
      <h1 class="ai-assistant-title">Assistant</h1>
      <p class="ai-assistant-bound" data-role="bound">No todo bound</p>
    </header>
    <div class="ai-assistant-messages" data-role="messages" aria-live="polite"></div>
    <form class="ai-assistant-composer" data-role="form">
      <textarea class="ai-assistant-input" data-role="input" rows="2" placeholder="Message for this todo…" disabled></textarea>
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

  function setComposerEnabled(enabled) {
    input.disabled = !enabled;
    sendBtn.disabled = !enabled || sending;
  }

  function applyBinding(payload) {
    if (!payload || typeof payload !== 'object') return;
    sessionId = String(payload.session_id || '');
    boundMasterTaskId = String(payload.bound_master_task_id || '');
    boundTitle = String(payload.bound_title || '');
    const busy = Boolean(payload.busy);
    messages = [];
    if (busy && payload.reply_text) {
      messages.push({ role: 'notice', text: String(payload.reply_text) });
    }
    boundEl.textContent = boundTitle
      ? `Bound: ${boundTitle}`
      : sessionId
        ? 'Session open'
        : 'No todo bound';
    setComposerEnabled(Boolean(sessionId) && !busy);
    renderMessages();
  }

  function pushNotice(text, error = false) {
    messages.push({ role: 'notice', text, error });
    renderMessages();
  }

  async function sendMessage(text) {
    const invoke = getTauriInvoke();
    if (!invoke) {
      pushNotice('Tauri invoke unavailable', true);
      return;
    }
    if (!sessionId) {
      pushNotice('No todo session bound', true);
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
        masterTaskId: boundMasterTaskId || null,
      });
      // Drop the local "Working…" notice (not a Session turn).
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
          if (result.bound_title) {
            boundTitle = String(result.bound_title);
            boundEl.textContent = `Bound: ${boundTitle}`;
          }
        }
      }
      renderMessages();
    } catch (err) {
      messages = messages.filter((m) => !(m.role === 'notice' && m.text === 'Working…'));
      pushNotice(err?.message ? String(err.message) : 'Failed to send', true);
    } finally {
      sending = false;
      setComposerEnabled(Boolean(sessionId));
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
      applyBinding(event?.payload);
    });
  }

  /** Heal first-open race: Host may emit before this window's listener is ready. */
  async function pullBindingFromHost() {
    const invoke = getTauriInvoke();
    if (!invoke) return;
    try {
      const state = await invoke('get_ai_assistant_binding');
      if (state && typeof state === 'object' && state.session_id) {
        applyBinding(state);
      }
    } catch {
      // Non-fatal; event path or later open may still bind.
    }
  }

  if (autoBind) {
    const fromUrl = bindingFromSearch();
    if (fromUrl) applyBinding(fromUrl);
    void (async () => {
      await bindOpenedListener();
      // After listen is armed, pull current Host binding (covers missed emit).
      await pullBindingFromHost();
    })();
  }

  function dispose() {
    if (typeof unlistenOpened === 'function') {
      void unlistenOpened();
    }
    // 关壳 ≠ Reset: notify Present shell close only; never invoke Binding Contract Reset.
    const invoke = getTauriInvoke();
    if (invoke) {
      void invoke('shell_close_ai_assistant').catch(() => {});
    }
    root.innerHTML = '';
  }

  return {
    dispose,
    applyBinding,
    getState: () => ({
      sessionId,
      boundMasterTaskId,
      boundTitle,
      messages,
      // Shell UX session binding ≠ Binding Contract bound (Present does not imply Set).
      presentSurface: PRESENT_SURFACE,
    }),
  };
}

const bootstrapRoot = document.getElementById('ai-assistant-root');
if (bootstrapRoot) {
  mountAiAssistant(bootstrapRoot);
}
