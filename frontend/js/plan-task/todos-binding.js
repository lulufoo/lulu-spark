/**
 * Todos Binding Contract consumer (SK-1): assemble T-lift Tools/Prompt + C-min callbacks,
 * Set/Reset via Host Binding Contract. Host does not assemble tools/prompt for Todos.
 */

/** T-lift opaque tool handles — aligned with `src-tauri/src/services/agent/tools.rs` TOOL_NAMES. */
export const TODOS_T_LIFT_TOOL_NAMES = Object.freeze([
  'get_plan',
  'list_sub_tasks',
  'add_sub_task',
  'update_sub_title',
  'update_master_title',
]);

/**
 * Maps existing `PLAN_ASSISTANT_SYSTEM_PROMPT` conventions (agent/mod.rs).
 * Owned by Todos consumer — Host must not fill or assemble this slot.
 */
export const TODOS_PLAN_ASSISTANT_PROMPT = `你是 lulu-workbench 的「计划任务」对话助手。当前会话只服务用户从计划页打开时所绑定的那一个计划。

## 你能做的事
1. 查看当前计划与子计划（通过工具）。
2. 给当前计划新增子计划。
3. 修改当前计划的主标题。
4. 修改某个子计划的标题。

## 你不能做的事
- 删除、完成/放弃子计划、改状态、批量操作、改 plan 正文、操作其它计划。
- 猜测用户没说清的目标就写入。
- 向用户索要或重复 API Key；配置在应用「设置」中完成。

## 工具使用
- 需要事实时先调用只读工具（get_plan / list_sub_tasks），再决定是否写入。
- 写入只能通过：add_sub_task、update_master_title、update_sub_title。
- 工具已绑定当前计划，不要编造 master_task_id，也不要要求用户提供计划 id。
- 改子标题前须能唯一确定 sub_task_id；不能确定就先 list/get，仍不能确定就提问澄清。

## 澄清
- 「改标题」未说明主标题还是子标题时，必须先问清再调用写入工具。
- 新增子计划缺少标题、或标题明显无效时，先问清再写入。
- 每次只问还缺的关键信息，简短。

## 回复风格
- 使用简洁中文。
- 写入成功：用一句话说明改了什么（主标题/子标题/新子计划），不要粘贴内部 JSON。
- 不支持的操作：明确说「目前不支持」，不要假装已完成。
- 执行失败：说明失败，可建议重试或去计划页手改；不要编造已成功。

## 结束
- 信息不足：澄清提问（仍保持在对话中）。
- 已完成或已说明不支持/失败：给出最终说明，等待用户下一句。`;

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

/** Assemble Binding body only (no Host call). callbacks registry may be empty `{}`. */
export function assembleTodosBindingBody(masterTaskId) {
  // masterTaskId retained for call-site compatibility; Host Agent P2 submits empty tools.
  void masterTaskId;
  return {
    // Interface layer: business session tools empty (capability dispatch removed in t3).
    tools: [],
    prompt: TODOS_PLAN_ASSISTANT_PROMPT,
    // B1: callbacks slot required; empty registry allowed. No Binding-top-level business fields.
    callbacks: {},
  };
}

function emitCallback(fn, payload) {
  if (typeof fn === 'function') {
    fn(payload);
  }
}

/**
 * Build T-lift Binding and Set via Host Binding Contract.
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

  const binding = assembleTodosBindingBody(masterTaskId);
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
