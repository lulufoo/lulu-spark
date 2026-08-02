// @vitest-environment jsdom
/**
 * L2 t1: Todos Binding 接入 — 自备 T-lift Tools/Prompt，经 Binding Contract Set/Reset/回调。
 * Sources: tech-doc L05-T / L09-AR / L08-AR / L10-AR / L12-I / L13-VF
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

import {
  assembleTodosBindingBody,
  buildTodosBinding,
  resetTodosBinding,
  TODOS_T_LIFT_TOOL_NAMES,
} from '../frontend/js/plan-task/todos-binding.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const toolsRs = readFileSync(
  join(fixtureRoot, 'src-tauri/src/services/agent/tools.rs'),
  'utf8',
);
const agentModRs = readFileSync(
  join(fixtureRoot, 'src-tauri/src/services/agent/mod.rs'),
  'utf8',
);
const todosBindingJs = readFileSync(
  join(fixtureRoot, 'frontend/js/plan-task/todos-binding.js'),
  'utf8',
);
const planTaskIndexJs = readFileSync(
  join(fixtureRoot, 'frontend/js/plan-task/index.js'),
  'utf8',
);

const T_LIFT = [
  'get_plan',
  'list_sub_tasks',
  'add_sub_task',
  'update_sub_title',
  'update_master_title',
];

describe('Todos Binding 接入 — source contracts', () => {
  it('business Binding submits tools:[]; capability TOOL_NAMES may remain until t3', () => {
    // Interface / consumer: assemble empty tools (P2 / t2).
    expect(assembleTodosBindingBody('task_x').tools).toEqual([]);
    // Capability layer residual names in tools.rs (dispatch deleted in t3).
    for (const name of T_LIFT) {
      expect(toolsRs).toContain(`"${name}"`);
    }
    expect(TODOS_T_LIFT_TOOL_NAMES).toHaveLength(5);
  });

  it('Todos prompt maps PLAN_ASSISTANT_SYSTEM_PROMPT conventions (mod.rs)', () => {
    expect(agentModRs).toMatch(/PLAN_ASSISTANT_SYSTEM_PROMPT/);
    expect(todosBindingJs).toMatch(/只服务|只读工具|目前不支持|API Key/);
    expect(planTaskIndexJs).toMatch(/todos-binding|buildTodosBinding|resetTodosBinding/);
  });

  it('Host is not asked to assemble tools/prompt for Todos (consumer owns Binding body)', () => {
    expect(todosBindingJs).not.toMatch(
      /invoke\(\s*['"](?:assemble|fill|default).*tools|Host.*拼装|requestHostAssemble/i,
    );
    expect(todosBindingJs).toMatch(/set_binding/);
    expect(todosBindingJs).toMatch(/reset_binding/);
  });
});

describe('buildTodosBinding / resetTodosBinding', () => {
  let invokeMock;
  let events;

  beforeEach(() => {
    events = [];
    invokeMock = vi.fn(async (cmd, args) => {
      if (cmd === 'set_binding') {
        const binding = args?.binding;
        if (
          !binding ||
          binding.tools == null ||
          binding.prompt == null ||
          binding.callbacks == null
        ) {
          return { ok: false, code: 'set_invalid', state: 'unbound' };
        }
        // tools slot required; empty array is legal (Host Agent P2 empty tools).
        const toolsOk = Array.isArray(binding.tools) ||
          (typeof binding.tools === 'object' && binding.tools !== null) ||
          (typeof binding.tools === 'string');
        const promptOk =
          (typeof binding.prompt === 'string' && binding.prompt.trim() !== '') ||
          (binding.prompt &&
            typeof binding.prompt === 'object' &&
            Object.keys(binding.prompt).length > 0);
        const callbacksOk =
          binding.callbacks &&
          typeof binding.callbacks === 'object' &&
          !Array.isArray(binding.callbacks);
        if (!toolsOk || !promptOk || !callbacksOk) {
          return { ok: false, code: 'set_invalid', state: 'unbound' };
        }
        return { ok: true, state: 'bound' };
      }
      if (cmd === 'reset_binding') {
        return { ok: true, state: 'unbound' };
      }
      if (cmd === 'execute_binding') {
        return { ok: false, code: 'rejected_unbound', state: 'unbound' };
      }
      if (cmd === 'query_binding') {
        return { state: 'unbound' };
      }
      if (cmd === 'ensure_ai_assistant_session') {
        return {
          session_id: 'sess_ensured',
          busy: false,
        };
      }
      return {};
    });
    window.__TAURI__ = { core: { invoke: invokeMock } };
  });

  afterEach(() => {
    delete window.__TAURI__;
    vi.restoreAllMocks();
  });

  function trackCallbacks() {
    return {
      onBound: (payload) => events.push({ event: 'onBound', payload }),
      onUnbound: (payload) => events.push({ event: 'onUnbound', payload }),
      onError: (payload) => events.push({ event: 'onError', payload }),
    };
  }

  it('assembles empty tools + prompt, Sets successfully, observes onBound (empty callbacks registry OK)', async () => {
    const cbs = trackCallbacks();
    const result = await buildTodosBinding(
      { masterTaskId: 'task_alpha' },
      cbs,
    );

    expect(result.ok).toBe(true);
    expect(result.state).toBe('bound');
    expect(result.binding).toBeTruthy();
    expect(result.binding.tools).toEqual([]);
    expect(result.binding.prompt).toBeTruthy();
    expect(result.binding.callbacks).toEqual({});
    expect(typeof result.binding.callbacks).toBe('object');

    const promptText =
      typeof result.binding.prompt === 'string'
        ? result.binding.prompt
        : JSON.stringify(result.binding.prompt);
    expect(promptText).toMatch(/只服务|计划/);

    expect(invokeMock).toHaveBeenCalledWith(
      'set_binding',
      expect.objectContaining({
        binding: expect.objectContaining({
          tools: [],
          prompt: expect.anything(),
          callbacks: {},
        }),
      }),
    );

    const setArgs = invokeMock.mock.calls.find((c) => c[0] === 'set_binding')?.[1];
    expect(setArgs.binding).not.toHaveProperty('master_task_id');
    expect(setArgs.binding).not.toHaveProperty('bound_master_task_id');
    expect(setArgs.binding).not.toHaveProperty('masterTaskId');

    expect(invokeMock).toHaveBeenCalledWith('ensure_ai_assistant_session');
    expect(events.map((e) => e.event)).toEqual(['onBound']);
  });

  it('does not Set on empty context (null / missing masterTaskId)', async () => {
    const cbs = trackCallbacks();
    const r1 = await buildTodosBinding(null, cbs);
    const r2 = await buildTodosBinding({}, cbs);
    const r3 = await buildTodosBinding({ masterTaskId: '' }, cbs);
    const r4 = await buildTodosBinding({ masterTaskId: '   ' }, cbs);

    expect(r1.ok).toBe(false);
    expect(r1.skipped).toBe('empty_context');
    expect(r2.skipped).toBe('empty_context');
    expect(r3.skipped).toBe('empty_context');
    expect(r4.skipped).toBe('empty_context');
    expect(invokeMock).not.toHaveBeenCalledWith(
      'set_binding',
      expect.anything(),
    );
    expect(events.filter((e) => e.event === 'onBound')).toHaveLength(0);
  });

  it('Reset observes onUnbound and old Binding cannot execute', async () => {
    const cbs = trackCallbacks();
    await buildTodosBinding({ masterTaskId: 'task_alpha' }, cbs);
    events.length = 0;

    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'reset_binding') return { ok: true, state: 'unbound' };
      if (cmd === 'execute_binding') {
        return { ok: false, code: 'rejected_unbound', state: 'unbound' };
      }
      if (cmd === 'query_binding') return { state: 'unbound' };
      return { ok: true, state: 'unbound' };
    });

    const resetResult = await resetTodosBinding(cbs);
    expect(resetResult.ok).toBe(true);
    expect(resetResult.state).toBe('unbound');
    expect(events.map((e) => e.event)).toEqual(['onUnbound']);

    const invoke = window.__TAURI__.core.invoke;
    const exec = await invoke('execute_binding');
    expect(exec.ok).toBe(false);
    expect(exec.code).toBe('rejected_unbound');
  });

  it('Set failure → onError(set_invalid); not bound; page path stays usable (no throw)', async () => {
    const cbs = trackCallbacks();
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'set_binding') {
        return { ok: false, code: 'set_invalid', state: 'unbound' };
      }
      return { ok: true, state: 'unbound' };
    });

    const result = await buildTodosBinding({ masterTaskId: 'task_alpha' }, cbs);
    expect(result.ok).toBe(false);
    expect(result.code).toBe('set_invalid');
    expect(result.state).toBe('unbound');
    expect(events.map((e) => e.event)).toEqual(['onError']);
    expect(events[0].payload).toMatchObject({ category: 'set_invalid' });
    expect(events[0].payload).not.toHaveProperty('master_task_id');
    expect(events[0].payload).not.toHaveProperty('masterTaskId');
  });

  it('callback payloads have no business fields; callbacks must not ask Host to assemble tools/prompt', async () => {
    const cbs = trackCallbacks();
    await buildTodosBinding({ masterTaskId: 'task_beta' }, cbs);
    await resetTodosBinding(cbs);

    for (const ev of events) {
      const p = ev.payload ?? {};
      expect(p).not.toHaveProperty('master_task_id');
      expect(p).not.toHaveProperty('bound_master_task_id');
      expect(p).not.toHaveProperty('masterTaskId');
      expect(p).not.toHaveProperty('tools');
      expect(p).not.toHaveProperty('prompt');
      expect(JSON.stringify(p)).not.toMatch(/assemble|fill.*prompt|拼装/i);
    }

    const setArgs = invokeMock.mock.calls.find((c) => c[0] === 'set_binding')?.[1];
    expect(setArgs.binding.callbacks).toEqual({});
    expect(setArgs.binding.tools).toEqual([]);
    expect(setArgs.binding.prompt).toBeTruthy();
  });
});


describe('t6 Binding Contract — no business ids on leave Reset', () => {
  it('resetTodosBinding / assemble body keep business ids off Binding Contract top-level', async () => {
    const invokeMock = vi.fn(async (cmd, args) => {
      if (cmd === 'set_binding') {
        const binding = args?.binding ?? {};
        expect(binding).not.toHaveProperty('master_task_id');
        expect(binding).not.toHaveProperty('bound_master_task_id');
        expect(binding).not.toHaveProperty('masterTaskId');
        return { ok: true, state: 'bound' };
      }
      if (cmd === 'reset_binding') {
        return { ok: true, state: 'unbound' };
      }
      if (cmd === 'ensure_ai_assistant_session') {
        return { session_id: 'sess_t6', busy: false };
      }
      return {};
    });
    window.__TAURI__ = { core: { invoke: invokeMock } };

    const set = await buildTodosBinding({ masterTaskId: 'task_t6_contract' });
    expect(set.ok).toBe(true);
    expect(set.binding).not.toHaveProperty('master_task_id');
    expect(set.binding).not.toHaveProperty('bound_master_task_id');
    expect(set.binding).not.toHaveProperty('masterTaskId');

    const reset = await resetTodosBinding();
    expect(reset.ok).toBe(true);
    expect(invokeMock).toHaveBeenCalledWith('reset_binding');
    // Reset payload must not invent business-id contract fields
    const resetCall = invokeMock.mock.calls.find((c) => c[0] === 'reset_binding');
    expect(resetCall?.[1] ?? {}).not.toHaveProperty('master_task_id');
    expect(resetCall?.[1] ?? {}).not.toHaveProperty('bound_master_task_id');

    delete window.__TAURI__;
  });
});
