/**
 * App Binding consumer: one process-level key-only Set for the spark surface.
 * Host looks up MCP Server config; consumer does not assemble tools/prompt/callbacks
 * or select an engine. No Reset helper — Binding is not tied to page enter/leave.
 */

/** Seeded business key — aligned with Host `mcp_host::registry::SEEDED_BUSINESS_KEY`. */
export const SPARK_BUSINESS_KEY = 'spark';

function getTauriInvoke() {
  if (typeof window === 'undefined') return null;
  const invoke =
    window.__TAURI__?.core?.invoke || window.__TAURI_INTERNALS__?.invoke;
  return typeof invoke === 'function' ? invoke : null;
}

type BindingCallbacks = {
  onBound?: (payload?: unknown) => void;
  onUnbound?: (payload?: unknown) => void;
  onError?: (payload?: unknown) => void;
};

type SetBindingResult = {
  ok?: boolean;
  code?: string;
  state?: string;
};

function emitCallback(fn: ((payload?: unknown) => void) | undefined, payload: unknown) {
  if (typeof fn === 'function') {
    fn(payload);
  }
}

/**
 * Set key-only spark Binding via Host Binding Contract.
 * Observes onBound / onError. Does not create a chat session — Set clears the
 * live id; Home starts a conversation with + or the first send.
 *
 * @param {{ onBound?: Function, onUnbound?: Function, onError?: Function }} [callbacks]
 */
export async function setSparkBinding(hooks: BindingCallbacks = {}) {
  const binding = { key: SPARK_BUSINESS_KEY };
  const invoke = getTauriInvoke();
  console.info('[DEBUG-assistant] setSparkBinding', {
    hasInvoke: Boolean(invoke),
    hasTauri: Boolean(typeof window !== 'undefined' && window.__TAURI__),
    hasInternals: Boolean(typeof window !== 'undefined' && window.__TAURI_INTERNALS__),
  });
  if (!invoke) {
    console.info('[DEBUG-assistant] setSparkBinding skip: no invoke');
    emitCallback(hooks.onError, { category: 'set_invalid' });
    return { ok: false, code: 'set_invalid', state: 'unbound', binding };
  }

  let result: SetBindingResult | undefined;
  try {
    result = (await invoke('set_binding', { binding })) as SetBindingResult;
    console.info('[DEBUG-assistant] setSparkBinding host result', result);
  } catch (err) {
    console.info('[DEBUG-assistant] setSparkBinding invoke threw', err);
    emitCallback(hooks.onError, { category: 'set_invalid' });
    return { ok: false, code: 'set_invalid', state: 'unbound', binding };
  }

  if (result && result.ok === true) {
    emitCallback(hooks.onBound, {});
    return {
      ok: true,
      state: result.state ?? 'bound',
      binding,
    };
  }

  const code =
    result && typeof result.code === 'string' ? result.code : 'set_invalid';
  emitCallback(hooks.onError, { category: code });
  return {
    ok: false,
    code,
    state: result?.state ?? 'unbound',
    binding,
  };
}
