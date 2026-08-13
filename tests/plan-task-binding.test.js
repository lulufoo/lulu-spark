// @vitest-environment jsdom
/**
 * L2 t3: Todos Binding 调用面只提交业务 key（key-only Set）。
 * Sources: tech-doc L11-T T3 / L09-I #4 / L13-VF AC5
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

import {
  assembleTodosBindingBody,
  buildTodosBinding,
  resetTodosBinding,
  TODOS_BUSINESS_KEY,
} from '../frontend/js/plan-task/todos-binding.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const todosBindingJs = readFileSync(
  join(fixtureRoot, 'frontend/js/plan-task/todos-binding.js'),
  'utf8',
);
const planTaskIndexJs = readFileSync(
  join(fixtureRoot, 'frontend/js/plan-task/index.js'),
  'utf8',
);

/** Host mock aligned with t2 public key-only Binding boundary. */
function acceptKeyOnlySet(binding) {
  if (!binding || typeof binding !== 'object') {
    return { ok: false, code: 'set_invalid', state: 'unbound' };
  }
  if (
    binding.tools != null ||
    binding.prompt != null ||
    binding.callbacks != null
  ) {
    return { ok: false, code: 'set_invalid', state: 'unbound' };
  }
  if (
    binding.engine != null ||
    binding.engine_type != null ||
    binding.engineType != null
  ) {
    return { ok: false, code: 'set_invalid', state: 'unbound' };
  }
  if (typeof binding.key !== 'string' || !binding.key.trim()) {
    return { ok: false, code: 'set_invalid', state: 'unbound' };
  }
  return { ok: true, state: 'bound' };
}

describe('Todos Binding 调用面 — source contracts (key-only)', () => {
  it('exports seeded business key todo_task for Set', () => {
    expect(TODOS_BUSINESS_KEY).toBe('todo_task');
    expect(todosBindingJs).toMatch(/todo_task/);
    expect(planTaskIndexJs).toMatch(
      /todos-binding|buildTodosBinding|resetTodosBinding/,
    );
  });

  it('production assemble/Set path has no tools/prompt/callbacks payload assembly', () => {
    expect(todosBindingJs).toMatch(/set_binding/);
    expect(todosBindingJs).toMatch(/reset_binding/);
    // assembleTodosBindingBody must not build legacy MCP payload fields
    const assembleIdx = todosBindingJs.indexOf('function assembleTodosBindingBody');
    expect(assembleIdx).toBeGreaterThanOrEqual(0);
    const assembleSlice = todosBindingJs.slice(
      assembleIdx,
      assembleIdx + 400,
    );
    expect(assembleSlice).not.toMatch(/\btools\s*:/);
    expect(assembleSlice).not.toMatch(/\bprompt\s*:/);
    expect(assembleSlice).not.toMatch(/\bcallbacks\s*:/);
    expect(assembleSlice).toMatch(/\bkey\b/);
  });

  it('call surface has no engine selection parameters', () => {
    expect(todosBindingJs).not.toMatch(/\bengineType\b|\bengine_type\b/);
    expect(todosBindingJs).not.toMatch(
      /invoke\(\s*['"]set_binding['"][\s\S]*engine/,
    );
  });
});

describe('assembleTodosBindingBody', () => {
  it('returns key-only payload with business key', () => {
    const body = assembleTodosBindingBody({ masterTaskId: 'task_alpha' });
    expect(body).toEqual({ key: 'todo_task' });
    expect(body).not.toHaveProperty('tools');
    expect(body).not.toHaveProperty('prompt');
    expect(body).not.toHaveProperty('callbacks');
    expect(body).not.toHaveProperty('engine');
    expect(body).not.toHaveProperty('engine_type');
    expect(body).not.toHaveProperty('engineType');
    expect(body).not.toHaveProperty('master_task_id');
    expect(body).not.toHaveProperty('masterTaskId');
  });
});

describe('buildTodosBinding / resetTodosBinding', () => {
  let invokeMock;
  let events;

  beforeEach(() => {
    events = [];
    invokeMock = vi.fn(async (cmd, args) => {
      if (cmd === 'set_binding') {
        return acceptKeyOnlySet(args?.binding);
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

  it('Sets with key-only payload, observes onBound; no MCP tools/prompt/callbacks', async () => {
    const cbs = trackCallbacks();
    const result = await buildTodosBinding(
      { masterTaskId: 'task_alpha' },
      cbs,
    );

    expect(result.ok).toBe(true);
    expect(result.state).toBe('bound');
    expect(result.binding).toEqual({ key: 'todo_task' });
    expect(result.binding).not.toHaveProperty('tools');
    expect(result.binding).not.toHaveProperty('prompt');
    expect(result.binding).not.toHaveProperty('callbacks');

    expect(invokeMock).toHaveBeenCalledWith(
      'set_binding',
      expect.objectContaining({
        binding: { key: 'todo_task' },
      }),
    );

    const setArgs = invokeMock.mock.calls.find((c) => c[0] === 'set_binding')?.[1];
    expect(Object.keys(setArgs.binding).sort()).toEqual(['key']);
    expect(setArgs.binding).not.toHaveProperty('master_task_id');
    expect(setArgs.binding).not.toHaveProperty('bound_master_task_id');
    expect(setArgs.binding).not.toHaveProperty('masterTaskId');
    expect(setArgs.binding).not.toHaveProperty('engine');
    expect(setArgs.binding).not.toHaveProperty('engine_type');
    expect(setArgs.binding).not.toHaveProperty('engineType');

    expect(invokeMock).toHaveBeenCalledWith('ensure_ai_assistant_session');
    expect(events.map((e) => e.event)).toEqual(['onBound']);
  });

  it('Sets key-only binding without live instance context', async () => {
    const cbs = trackCallbacks();
    const r1 = await buildTodosBinding(null, cbs);
    const r2 = await buildTodosBinding({}, cbs);
    const r3 = await buildTodosBinding({ masterTaskId: '' }, cbs);
    const r4 = await buildTodosBinding({ masterTaskId: '   ' }, cbs);

    expect([r1, r2, r3, r4].map((result) => result.ok)).toEqual([
      true,
      true,
      true,
      true,
    ]);
    for (const result of [r1, r2, r3, r4]) {
      expect(result.binding).toEqual({ key: 'todo_task' });
    }

    const setCalls = invokeMock.mock.calls.filter(
      ([command]) => command === 'set_binding',
    );
    expect(setCalls).toHaveLength(4);
    for (const [, args] of setCalls) {
      expect(args).toEqual({ binding: { key: 'todo_task' } });
    }
    expect(events.map((e) => e.event)).toEqual([
      'onBound',
      'onBound',
      'onBound',
      'onBound',
    ]);
  });

  it('Reset observes onUnbound; reset_binding carries no config/engine args', async () => {
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

    const resetCall = invokeMock.mock.calls.find((c) => c[0] === 'reset_binding');
    expect(resetCall).toBeTruthy();
    // invoke('reset_binding') with no config/engine payload
    expect(resetCall.length).toBe(1);
    expect(resetCall?.[1] ?? {}).not.toHaveProperty('key');
    expect(resetCall?.[1] ?? {}).not.toHaveProperty('tools');
    expect(resetCall?.[1] ?? {}).not.toHaveProperty('prompt');
    expect(resetCall?.[1] ?? {}).not.toHaveProperty('callbacks');
    expect(resetCall?.[1] ?? {}).not.toHaveProperty('engine');

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

  it('legacy tools/prompt/callbacks shape is not a successful business Set path', async () => {
    const cbs = trackCallbacks();
    const result = await buildTodosBinding({ masterTaskId: 'task_beta' }, cbs);
    expect(result.ok).toBe(true);

    const setArgs = invokeMock.mock.calls.find((c) => c[0] === 'set_binding')?.[1];
    expect(setArgs.binding).toEqual({ key: 'todo_task' });

    // Host boundary rejects legacy shape (t2); consumer must not submit it
    const legacyRejected = acceptKeyOnlySet({
      tools: [{ name: 'get_plan' }],
      prompt: 'legacy',
      callbacks: {},
    });
    expect(legacyRejected).toEqual({
      ok: false,
      code: 'set_invalid',
      state: 'unbound',
    });
  });

  it('callback payloads have no business/config/engine fields', async () => {
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
      expect(p).not.toHaveProperty('engine');
      expect(JSON.stringify(p)).not.toMatch(/assemble|fill.*prompt|拼装/i);
    }
  });
});
