/**
 * Todos Binding Contract consumer: Set/Reset via Host Binding Contract with
 * key-only payload. Host looks up MCP Server config; consumer does not assemble
 * tools/prompt/callbacks or select an engine.
 */

/** Seeded business key — aligned with Host `mcp_server_registry::SEEDED_BUSINESS_KEY`. */
export const TODOS_BUSINESS_KEY = 'todo_task';

function getTauriInvoke() {
  if (typeof window === 'undefined') return null;
  const invoke =
    window.__TAURI__?.core?.invoke || window.__TAURI_INTERNALS__?.invoke;
  return typeof invoke === 'function' ? invoke : null;
}

function masterIdFromContext(masterContext) {
  if (!masterContext || typeof masterContext !== 'object') return null;
  const raw = masterContext.masterTaskId;
  if (typeof raw !== 'string') return null;
  const id = raw.trim();
  return id ? id : null;
}

/** Assemble Binding body only (no Host call). Key-only public Set contract. */
export function assembleTodosBindingBody(_masterContext) {
  return { key: TODOS_BUSINESS_KEY };
}

function emitCallback(fn, payload) {
  if (typeof fn === 'function') {
    fn(payload);
  }
}

/**
 * Build key-only Binding and Set via Host Binding Contract.
 * Does not Set on empty context. Observes onBound / onError (C-min).
 *
 * @param {{ masterTaskId: string } | null} masterContext
 * @param {{ onBound?: Function, onUnbound?: Function, onError?: Function }} [callbacks]
 */
export async function buildTodosBinding(masterContext, callbacks = {}) {
  const masterTaskId = masterIdFromContext(masterContext);
  if (!masterTaskId) {
    return { ok: false, skipped: 'empty_context', state: 'unbound' };
  }

  const binding = assembleTodosBindingBody(masterContext);
  const invoke = getTauriInvoke();
  if (!invoke) {
    const payload = { category: 'set_invalid' };
    emitCallback(callbacks.onError, payload);
    return { ok: false, code: 'set_invalid', state: 'unbound', binding };
  }

  let result;
  try {
    result = await invoke('set_binding', { binding });
  } catch {
    const payload = { category: 'set_invalid' };
    emitCallback(callbacks.onError, payload);
    return { ok: false, code: 'set_invalid', state: 'unbound', binding };
  }

  if (result && result.ok === true) {
    // Chat session for turn history only — no master write (Present≠Set; Host≠todo id).
    try {
      await invoke('ensure_ai_assistant_session');
    } catch {
      // Binding Contract Set already succeeded; session heal may retry on send.
    }
    emitCallback(callbacks.onBound, {});
    return {
      ok: true,
      state: result.state ?? 'bound',
      binding,
    };
  }

  const code =
    result && typeof result.code === 'string' ? result.code : 'set_invalid';
  emitCallback(callbacks.onError, { category: code });
  return {
    ok: false,
    code,
    state: result?.state ?? 'unbound',
    binding,
  };
}

/**
 * Reset Host Binding → unbound. Observes onUnbound (C-min).
 *
 * @param {{ onBound?: Function, onUnbound?: Function, onError?: Function }} [callbacks]
 */
export async function resetTodosBinding(callbacks = {}) {
  const invoke = getTauriInvoke();
  if (!invoke) {
    emitCallback(callbacks.onUnbound, {});
    return { ok: true, state: 'unbound' };
  }

  let result;
  try {
    result = await invoke('reset_binding');
  } catch {
    emitCallback(callbacks.onUnbound, {});
    return { ok: true, state: 'unbound' };
  }

  emitCallback(callbacks.onUnbound, {});
  return {
    ok: true,
    state: result?.state ?? 'unbound',
  };
}
