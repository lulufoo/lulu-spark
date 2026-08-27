// @vitest-environment jsdom
/**
 * t2: App Binding helper is workbench-only Set (no Reset helper, no old keys).
 * Sources: tech-doc C6-I / C7_4a-T / C7_4b-T
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

import {
  setWorkbenchBinding,
  WORKBENCH_BUSINESS_KEY,
} from '../frontend/js/todo-task/todos-binding.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const todosBindingJs = readFileSync(
  join(fixtureRoot, 'frontend/js/todo-task/todos-binding.js'),
  'utf8',
);
const todoTaskIndexJs = readFileSync(
  join(fixtureRoot, 'frontend/js/todo-task/index.js'),
  'utf8',
);

const DELETED_HELPERS = [
  'TODOS_BUSINESS_KEY',
  'NOTES_BUSINESS_KEY',
  'assembleTodosBindingBody',
  'assembleNotesBindingBody',
  'buildTodosBinding',
  'buildNotesBinding',
  'resetTodosBinding',
  'resetNotesBinding',
];

/** Host mock aligned with t1: only workbench key-only Set succeeds. */
function acceptWorkbenchKeyOnlySet(binding) {
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
  if (binding.key === 'notes' || binding.key === 'todo_task') {
    return { ok: false, code: 'set_invalid', state: 'unbound' };
  }
  if (binding.key !== 'workbench') {
    return { ok: false, code: 'set_invalid', state: 'unbound' };
  }
  return { ok: true, state: 'bound' };
}

describe('Workbench Binding helper — source contracts', () => {
  it('exports only WORKBENCH_BUSINESS_KEY and setWorkbenchBinding', async () => {
    expect(WORKBENCH_BUSINESS_KEY).toBe('workbench');
    expect(typeof setWorkbenchBinding).toBe('function');
    expect(setWorkbenchBinding.length).toBeLessThanOrEqual(1);

    const bindingMod = await import('../frontend/js/todo-task/todos-binding.js');
    expect(Object.keys(bindingMod).sort()).toEqual([
      'WORKBENCH_BUSINESS_KEY',
      'setWorkbenchBinding',
    ]);
    for (const name of DELETED_HELPERS) {
      expect(bindingMod[name], `${name} must be deleted`).toBeUndefined();
      expect(todosBindingJs).not.toMatch(new RegExp(`\\b${name}\\b`));
    }
    expect(todosBindingJs).not.toMatch(/resetWorkbenchBinding|reset_binding/);
  });

  it('index.js does not re-export deleted Binding helpers', () => {
    for (const name of DELETED_HELPERS) {
      expect(todoTaskIndexJs).not.toMatch(new RegExp(`\\b${name}\\b`));
    }
  });

  it('production Set path has no tools/prompt/callbacks payload assembly', () => {
    expect(todosBindingJs).toMatch(/set_binding/);
    const setIdx = todosBindingJs.indexOf('function setWorkbenchBinding');
    expect(setIdx).toBeGreaterThanOrEqual(0);
    const setSlice = todosBindingJs.slice(setIdx, setIdx + 500);
    expect(setSlice).not.toMatch(/\btools\s*:/);
    expect(setSlice).not.toMatch(/\bprompt\s*:/);
    expect(setSlice).not.toMatch(/\bcallbacks\s*:/);
    expect(setSlice).toMatch(/\bkey\b/);
  });

  it('call surface has no engine selection or master/route parameters', () => {
    expect(todosBindingJs).not.toMatch(/\bengineType\b|\bengine_type\b/);
    expect(todosBindingJs).not.toMatch(/\bmasterTaskId\b|\bmaster_task_id\b/);
    expect(todosBindingJs).not.toMatch(
      /invoke\(\s*['"]set_binding['"][\s\S]*engine/,
    );
    expect(todosBindingJs).not.toMatch(/['"]notes['"]|['"]todo_task['"]/);
  });
});

describe('setWorkbenchBinding', () => {
  let invokeMock;
  let events;

  beforeEach(() => {
    events = [];
    invokeMock = vi.fn(async (cmd, args) => {
      if (cmd === 'set_binding') {
        return acceptWorkbenchKeyOnlySet(args?.binding);
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

  it('Sets with key-only workbench payload, observes onBound; then ensure session', async () => {
    const cbs = trackCallbacks();
    const result = await setWorkbenchBinding(cbs);

    expect(result.ok).toBe(true);
    expect(result.state).toBe('bound');
    expect(result.binding).toEqual({ key: 'workbench' });
    expect(result.binding).not.toHaveProperty('tools');
    expect(result.binding).not.toHaveProperty('prompt');
    expect(result.binding).not.toHaveProperty('callbacks');

    expect(invokeMock).toHaveBeenCalledWith('set_binding', {
      binding: { key: 'workbench' },
    });

    const setArgs = invokeMock.mock.calls.find((c) => c[0] === 'set_binding')?.[1];
    expect(Object.keys(setArgs.binding).sort()).toEqual(['key']);
    expect(setArgs.binding).not.toHaveProperty('master_task_id');
    expect(setArgs.binding).not.toHaveProperty('bound_master_task_id');
    expect(setArgs.binding).not.toHaveProperty('masterTaskId');
    expect(setArgs.binding).not.toHaveProperty('engine');
    expect(setArgs.binding).not.toHaveProperty('engine_type');
    expect(setArgs.binding).not.toHaveProperty('engineType');

    expect(invokeMock).not.toHaveBeenCalledWith('ensure_ai_assistant_session');
    expect(events.map((e) => e.event)).toEqual(['onBound']);
    expect(invokeMock).not.toHaveBeenCalledWith('reset_binding');
  });

  it('does not write master / route / instance id into Binding', async () => {
    const cbs = trackCallbacks();
    const result = await setWorkbenchBinding({
      ...cbs,
      masterTaskId: 'task_alpha',
      route: '#/todo-tasks',
    });

    expect(result.ok).toBe(true);
    expect(result.binding).toEqual({ key: 'workbench' });
    const setArgs = invokeMock.mock.calls.find((c) => c[0] === 'set_binding')?.[1];
    expect(setArgs).toEqual({ binding: { key: 'workbench' } });
    expect(JSON.stringify(setArgs.binding)).not.toMatch(
      /task_alpha|#\/todo-tasks|master/i,
    );
  });

  it('does not create a chat session after a successful Set', async () => {
    const cbs = trackCallbacks();
    const result = await setWorkbenchBinding(cbs);
    expect(result.ok).toBe(true);
    expect(result.state).toBe('bound');
    expect(invokeMock).toHaveBeenCalledWith('set_binding', {
      binding: { key: 'workbench' },
    });
    expect(invokeMock).not.toHaveBeenCalledWith('ensure_ai_assistant_session');
    expect(events.map((e) => e.event)).toEqual(['onBound']);
  });

  it('Host rejects notes|todo_task keys; helper never sends those keys', async () => {
    const notesRejected = acceptWorkbenchKeyOnlySet({ key: 'notes' });
    const todosRejected = acceptWorkbenchKeyOnlySet({ key: 'todo_task' });
    expect(notesRejected).toEqual({
      ok: false,
      code: 'set_invalid',
      state: 'unbound',
    });
    expect(todosRejected).toEqual({
      ok: false,
      code: 'set_invalid',
      state: 'unbound',
    });

    await setWorkbenchBinding(trackCallbacks());
    const setCalls = invokeMock.mock.calls.filter(([cmd]) => cmd === 'set_binding');
    expect(setCalls).toHaveLength(1);
    expect(setCalls[0][1].binding.key).toBe('workbench');
    expect(setCalls[0][1].binding.key).not.toBe('notes');
    expect(setCalls[0][1].binding.key).not.toBe('todo_task');
  });

  it('Set failure → onError(set_invalid); does not pretend workbench is bound', async () => {
    const cbs = trackCallbacks();
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'set_binding') {
        return { ok: false, code: 'set_invalid', state: 'unbound' };
      }
      return { ok: true, state: 'unbound' };
    });

    const result = await setWorkbenchBinding(cbs);
    expect(result.ok).toBe(false);
    expect(result.code).toBe('set_invalid');
    expect(result.state).toBe('unbound');
    expect(events.map((e) => e.event)).toEqual(['onError']);
    expect(events[0].payload).toMatchObject({ category: 'set_invalid' });
    expect(invokeMock).not.toHaveBeenCalledWith('ensure_ai_assistant_session');
  });

  it('set_binding throw → onError / ok:false; not bound', async () => {
    const cbs = trackCallbacks();
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'set_binding') {
        throw new Error('ipc failed');
      }
      return {};
    });

    const result = await setWorkbenchBinding(cbs);
    expect(result.ok).toBe(false);
    expect(result.code).toBe('set_invalid');
    expect(result.state).toBe('unbound');
    expect(events.map((e) => e.event)).toEqual(['onError']);
  });

  it('missing invoke → onError / ok:false; does not pretend workbench is bound', async () => {
    delete window.__TAURI__;
    const cbs = trackCallbacks();
    const result = await setWorkbenchBinding(cbs);
    expect(result.ok).toBe(false);
    expect(result.code).toBe('set_invalid');
    expect(result.state).toBe('unbound');
    expect(events.map((e) => e.event)).toEqual(['onError']);
  });

  it('legacy tools/prompt/callbacks shape is not a successful workbench Set path', async () => {
    const result = await setWorkbenchBinding(trackCallbacks());
    expect(result.ok).toBe(true);
    const setArgs = invokeMock.mock.calls.find((c) => c[0] === 'set_binding')?.[1];
    expect(setArgs.binding).toEqual({ key: 'workbench' });

    const legacyRejected = acceptWorkbenchKeyOnlySet({
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
    await setWorkbenchBinding(trackCallbacks());
    for (const ev of events) {
      const p = ev.payload ?? {};
      expect(p).not.toHaveProperty('master_task_id');
      expect(p).not.toHaveProperty('bound_master_task_id');
      expect(p).not.toHaveProperty('masterTaskId');
      expect(p).not.toHaveProperty('tools');
      expect(p).not.toHaveProperty('prompt');
      expect(p).not.toHaveProperty('engine');
    }
  });
});
