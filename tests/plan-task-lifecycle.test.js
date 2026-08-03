// @vitest-environment jsdom
/**
 * L2 t2: Todos page lifecycle — enter/selection Set, leave Reset; shell close ≠ Reset.
 * Sources: tech-doc L08-AR / L10-AR / L12-I#2 / L13-VF
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const getJsonMock = vi.fn();

vi.mock('../frontend/js/apiClient.js', async (importOriginal) => {
  const actual = await importOriginal();
  return {
    ...actual,
    resolveReadDriver: vi.fn(() => ({ getJson: getJsonMock })),
    createApiClient: (driver) => ({
      getJson: driver?.getJson ?? getJsonMock,
    }),
  };
});

import {
  createTodosPageLifecycle,
  mountPlanTaskSplit,
  onTodosPageEnter,
  onMasterSelectionChange,
  onTodosPageLeave,
  TODOS_EXPLICIT_LEAVE_RESET_CHAIN,
} from '../frontend/js/plan-task/index.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const planTaskIndexJs = readFileSync(
  join(fixtureRoot, 'frontend/js/plan-task/index.js'),
  'utf8',
);

const sampleMasters = [
  {
    master_task_id: 'task_alpha',
    title: 'Alpha Task',
    status: 'incomplete',
    created_at: '2026-07-01T10:00:00Z',
    sub_tasks: [
      {
        sub_task_id: 'task_alpha_sub_01',
        title: 'Alpha Sub A',
        status: 'incomplete',
        implicit: false,
        linked_archive_ids: [],
      },
    ],
  },
  {
    master_task_id: 'task_beta',
    title: 'Beta Task',
    status: 'incomplete',
    created_at: '2026-07-02T10:00:00Z',
    sub_tasks: [
      {
        sub_task_id: 'task_beta_sub_01',
        title: 'Beta Sub A',
        status: 'incomplete',
        implicit: false,
        linked_archive_ids: [],
      },
    ],
  },
];

describe('Todos page lifecycle — source contracts', () => {
  it('exports page lifecycle entry points from plan-task/index.js', () => {
    expect(typeof createTodosPageLifecycle).toBe('function');
    expect(typeof onTodosPageEnter).toBe('function');
    expect(typeof onMasterSelectionChange).toBe('function');
    expect(typeof onTodosPageLeave).toBe('function');
    expect(planTaskIndexJs).toMatch(/onTodosPageEnter|createTodosPageLifecycle/);
    expect(planTaskIndexJs).toMatch(/onMasterSelectionChange/);
    expect(planTaskIndexJs).toMatch(/onTodosPageLeave/);
  });

  it('dispose / leave path calls Reset; shell-close path must not call Reset', () => {
    expect(planTaskIndexJs).toMatch(/onTodosPageLeave|resetTodosBinding/);
    // Shell close must not be wired as leave/Reset trigger.
    expect(planTaskIndexJs).not.toMatch(
      /shell_close[\s\S]{0,120}onTodosPageLeave|onTodosPageLeave[\s\S]{0,120}shell_close/i,
    );
  });
});

describe('createTodosPageLifecycle', () => {
  let invokeMock;
  let hostBound;
  let hostBindingToken;
  let events;

  beforeEach(() => {
    events = [];
    hostBound = false;
    hostBindingToken = null;
    invokeMock = vi.fn(async (cmd, args) => {
      if (cmd === 'set_binding') {
        const binding = args?.binding;
        if (
          !binding ||
          binding.tools == null ||
          binding.prompt == null ||
          binding.callbacks == null
        ) {
          return { ok: false, code: 'set_invalid', state: hostBound ? 'bound' : 'unbound' };
        }
        hostBound = true;
        hostBindingToken = binding.__testToken ?? binding.prompt?.slice?.(0, 8) ?? 'set';
        return { ok: true, state: 'bound' };
      }
      if (cmd === 'reset_binding') {
        hostBound = false;
        hostBindingToken = null;
        return { ok: true, state: 'unbound' };
      }
      if (cmd === 'execute_binding') {
        if (!hostBound) {
          return { ok: false, code: 'rejected_unbound', state: 'unbound' };
        }
        return {
          ok: true,
          state: 'bound',
          applied_token: hostBindingToken,
        };
      }
      if (cmd === 'query_binding') {
        return { state: hostBound ? 'bound' : 'unbound' };
      }
      if (cmd === 'open_ai_assistant' || cmd === 'present_ai_assistant') {
        return { ok: true, window_label: 'ai-assistant' };
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

  it('enter with available selected master → Set → onBound', async () => {
    const life = createTodosPageLifecycle(trackCallbacks());
    const result = await life.onTodosPageEnter('task_alpha');

    expect(result.ok).toBe(true);
    expect(result.state).toBe('bound');
    expect(life.isBound()).toBe(true);
    expect(life.getBoundMasterId()).toBe('task_alpha');
    expect(events.map((e) => e.event)).toEqual(['onBound']);
    expect(invokeMock).toHaveBeenCalledWith(
      'set_binding',
      expect.objectContaining({ binding: expect.any(Object) }),
    );
  });

  it('enter without available selected master → no Set; first selection later → Set → onBound', async () => {
    const life = createTodosPageLifecycle(trackCallbacks());

    const deferred = await life.onTodosPageEnter('');
    expect(deferred.ok).toBe(false);
    expect(deferred.skipped).toBe('empty_context');
    expect(invokeMock).not.toHaveBeenCalledWith('set_binding', expect.anything());
    expect(events.filter((e) => e.event === 'onBound')).toHaveLength(0);

    const first = await life.onMasterSelectionChange('task_alpha');
    expect(first.ok).toBe(true);
    expect(life.isBound()).toBe(true);
    expect(life.getBoundMasterId()).toBe('task_alpha');
    expect(events.map((e) => e.event)).toEqual(['onBound']);
  });

  it('in-page master change → replace Set (bound→bound); execute uses new binding only', async () => {
    const life = createTodosPageLifecycle(trackCallbacks());
    await life.onTodosPageEnter('task_alpha');
    events.length = 0;

    let setCount = 0;
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'set_binding') {
        setCount += 1;
        hostBound = true;
        hostBindingToken = `tok_${setCount}`;
        return { ok: true, state: 'bound', __setCount: setCount };
      }
      if (cmd === 'execute_binding') {
        return { ok: true, state: 'bound', applied_token: hostBindingToken };
      }
      if (cmd === 'reset_binding') {
        hostBound = false;
        hostBindingToken = null;
        return { ok: true, state: 'unbound' };
      }
      return {};
    });

    await life.onTodosPageEnter('task_alpha');
    const tokenAfterAlpha = hostBindingToken;
    events.length = 0;

    const replaced = await life.onMasterSelectionChange('task_beta');
    expect(replaced.ok).toBe(true);
    expect(life.getBoundMasterId()).toBe('task_beta');
    expect(events.map((e) => e.event)).toEqual(['onBound']);
    expect(hostBindingToken).not.toBe(tokenAfterAlpha);

    const exec = await window.__TAURI__.core.invoke('execute_binding');
    expect(exec.ok).toBe(true);
    expect(exec.applied_token).toBe(hostBindingToken);
    expect(exec.applied_token).not.toBe(tokenAfterAlpha);

    // Must not force Present / open_ai_assistant on selection change.
    expect(invokeMock).not.toHaveBeenCalledWith(
      'open_ai_assistant',
      expect.anything(),
    );
    expect(invokeMock).not.toHaveBeenCalledWith(
      'present_ai_assistant',
      expect.anything(),
    );
  });

  it('leave / dispose → Reset → onUnbound', async () => {
    const life = createTodosPageLifecycle(trackCallbacks());
    await life.onTodosPageEnter('task_alpha');
    events.length = 0;

    const left = await life.onTodosPageLeave();
    expect(left.ok).toBe(true);
    expect(left.state).toBe('unbound');
    expect(life.isBound()).toBe(false);
    expect(life.getBoundMasterId()).toBe(null);
    expect(events.map((e) => e.event)).toEqual(['onUnbound']);
    expect(invokeMock).toHaveBeenCalledWith('reset_binding');

    const exec = await window.__TAURI__.core.invoke('execute_binding');
    expect(exec.ok).toBe(false);
    expect(exec.code).toBe('rejected_unbound');
  });

  it('shell close ≠ Reset: Binding kept; no onUnbound', async () => {
    const life = createTodosPageLifecycle(trackCallbacks());
    await life.onTodosPageEnter('task_alpha');
    events.length = 0;

    const close = life.notifyShellClose();
    expect(close.reset).toBe(false);
    expect(life.isBound()).toBe(true);
    expect(life.getBoundMasterId()).toBe('task_alpha');
    expect(events.filter((e) => e.event === 'onUnbound')).toHaveLength(0);
    expect(invokeMock).not.toHaveBeenCalledWith('reset_binding');

    const exec = await window.__TAURI__.core.invoke('execute_binding');
    expect(exec.ok).toBe(true);
  });

  it('replace Set failure keeps old Binding; onError observable; old binding still executable', async () => {
    const life = createTodosPageLifecycle(trackCallbacks());
    await life.onTodosPageEnter('task_alpha');
    const tokenOld = hostBindingToken;
    events.length = 0;

    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'set_binding') {
        // Host refuses replace — remains bound with previous tools/prompt.
        return { ok: false, code: 'set_invalid', state: 'bound' };
      }
      if (cmd === 'execute_binding') {
        return { ok: true, state: 'bound', applied_token: tokenOld };
      }
      if (cmd === 'reset_binding') {
        hostBound = false;
        return { ok: true, state: 'unbound' };
      }
      return {};
    });

    const failed = await life.onMasterSelectionChange('task_beta');
    expect(failed.ok).toBe(false);
    expect(failed.code).toBe('set_invalid');
    expect(life.isBound()).toBe(true);
    expect(life.getBoundMasterId()).toBe('task_alpha');
    expect(failed.retainedBinding).toBe(true);
    expect(events.map((e) => e.event)).toEqual(['onError']);
    expect(invokeMock).not.toHaveBeenCalledWith('reset_binding');

    const exec = await window.__TAURI__.core.invoke('execute_binding');
    expect(exec.ok).toBe(true);
    expect(exec.applied_token).toBe(tokenOld);
  });

  it('enter Set failure: page usable path (no throw); onError; not executable until later successful Set', async () => {
    const life = createTodosPageLifecycle(trackCallbacks());
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'set_binding') {
        return { ok: false, code: 'set_invalid', state: 'unbound' };
      }
      if (cmd === 'execute_binding') {
        return { ok: false, code: 'rejected_unbound', state: 'unbound' };
      }
      return { ok: true, state: 'unbound' };
    });

    await expect(life.onTodosPageEnter('task_alpha')).resolves.toMatchObject({
      ok: false,
      code: 'set_invalid',
    });
    expect(life.isBound()).toBe(false);
    expect(events.map((e) => e.event)).toEqual(['onError']);

    let exec = await window.__TAURI__.core.invoke('execute_binding');
    expect(exec.ok).toBe(false);
    expect(exec.code).toBe('rejected_unbound');

    // Recovery: re-select / retry Set succeeds → executable.
    events.length = 0;
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'set_binding') {
        hostBound = true;
        hostBindingToken = 'recovered';
        return { ok: true, state: 'bound' };
      }
      if (cmd === 'execute_binding') {
        return { ok: true, state: 'bound', applied_token: hostBindingToken };
      }
      return {};
    });

    const recovered = await life.onMasterSelectionChange('task_beta');
    expect(recovered.ok).toBe(true);
    expect(life.isBound()).toBe(true);
    expect(events.map((e) => e.event)).toEqual(['onBound']);
    exec = await window.__TAURI__.core.invoke('execute_binding');
    expect(exec.ok).toBe(true);
  });
});

describe('mountPlanTaskSplit wires page lifecycle', () => {
  let container;
  let invokeMock;
  let hostBound;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    getJsonMock.mockReset();
    getJsonMock.mockResolvedValue(sampleMasters);
    hostBound = false;

    invokeMock = vi.fn(async (cmd, args) => {
      if (cmd === 'set_binding') {
        const binding = args?.binding;
        if (!binding?.tools || !binding?.prompt || binding.callbacks == null) {
          return { ok: false, code: 'set_invalid', state: hostBound ? 'bound' : 'unbound' };
        }
        hostBound = true;
        return { ok: true, state: 'bound' };
      }
      if (cmd === 'reset_binding') {
        hostBound = false;
        return { ok: true, state: 'unbound' };
      }
      if (cmd === 'execute_binding') {
        return hostBound
          ? { ok: true, state: 'bound' }
          : { ok: false, code: 'rejected_unbound', state: 'unbound' };
      }
      if (cmd === 'open_ai_assistant') {
        return {
          session_id: 'sess_1',
          bound_master_task_id: args?.masterTaskId,
          window_label: 'ai-assistant',
          busy: false,
        };
      }
      return {};
    });

    window.__TAURI__ = {
      core: { invoke: invokeMock },
      event: { listen: vi.fn(async () => vi.fn()) },
    };
  });

  afterEach(() => {
    container?.remove();
    delete window.__TAURI__;
    vi.restoreAllMocks();
  });

  it('mount with selected master Sets; dispose Resets; selection change replaces Set without Present', async () => {
    const api = mountPlanTaskSplit(container, { masterId: 'task_alpha' });
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        'set_binding',
        expect.objectContaining({ binding: expect.any(Object) }),
      );
    });

    invokeMock.mockClear();
    const betaBtn = container.querySelector(
      '.plan-task-master-item[data-master-id="task_beta"]',
    );
    expect(betaBtn).toBeTruthy();
    betaBtn.click();

    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        'set_binding',
        expect.objectContaining({ binding: expect.any(Object) }),
      );
    });
    expect(invokeMock).not.toHaveBeenCalledWith(
      'open_ai_assistant',
      expect.anything(),
    );
    expect(invokeMock).not.toHaveBeenCalledWith('reset_binding');

    invokeMock.mockClear();
    api.dispose();
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('reset_binding');
    });
  });

  it('mount without selection chooses first displayed master and Sets', async () => {
    const api = mountPlanTaskSplit(container, { masterId: '' });
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        'set_binding',
        expect.objectContaining({ binding: expect.any(Object) }),
      );
    });

    const selected = container.querySelector(
      '.plan-task-master-item--selected',
    );
    expect(selected?.dataset.masterId).toBe('task_beta');
    const setCall = invokeMock.mock.calls.find(([command]) => command === 'set_binding');
    expect(setCall?.[1]?.binding.tools).toEqual([]);
    api.dispose();
  });
});


describe('t6 Todos leave still explicit Reset + layered leave wiring', () => {
  it('exports explicit leave→Reset chain (dispose→onTodosPageLeave→resetTodosBinding→reset_binding)', () => {
    expect(TODOS_EXPLICIT_LEAVE_RESET_CHAIN).toEqual([
      'frontend/js/plan-task/index.js::dispose',
      'frontend/js/plan-task/todos-lifecycle.js::onTodosPageLeave',
      'frontend/js/plan-task/todos-binding.js::resetTodosBinding',
      'src-tauri/src/services/agent/loop.rs::reset_binding',
    ]);
    expect(planTaskIndexJs).toMatch(/TODOS_EXPLICIT_LEAVE_RESET_CHAIN/);
    // dispose must call onTodosPageLeave (explicit Reset); defensive cut must not omit it
    expect(planTaskIndexJs).toMatch(
      /function dispose\(\)\s*\{[\s\S]*?onTodosPageLeave/,
    );
    // unbound UI refresh wired on lifecycle (Todos small-change ceiling)
    expect(planTaskIndexJs).toMatch(
      /createTodosPageLifecycle\(\s*\{[\s\S]*onUnbound/,
    );
  });

  it('dispose / unmount reliably Reset; second dispose is idempotent (no duplicate Reset)', async () => {
    const container = document.createElement('div');
    document.body.appendChild(container);
    getJsonMock.mockReset();
    getJsonMock.mockResolvedValue(sampleMasters);
    let hostBound = false;
    const invokeMock = vi.fn(async (cmd, args) => {
      if (cmd === 'set_binding') {
        const binding = args?.binding;
        if (!binding?.tools || !binding?.prompt || binding.callbacks == null) {
          return { ok: false, code: 'set_invalid', state: hostBound ? 'bound' : 'unbound' };
        }
        hostBound = true;
        return { ok: true, state: 'bound' };
      }
      if (cmd === 'reset_binding') {
        hostBound = false;
        return { ok: true, state: 'unbound' };
      }
      if (cmd === 'execute_binding') {
        return hostBound
          ? { ok: true, state: 'bound' }
          : { ok: false, code: 'rejected_unbound', state: 'unbound' };
      }
      if (cmd === 'present_ai_assistant' || cmd === 'open_ai_assistant') {
        return { ok: true, window_label: 'ai-assistant' };
      }
      if (cmd === 'ensure_ai_assistant_session') {
        return { session_id: 'sess_1', busy: false };
      }
      return {};
    });
    window.__TAURI__ = {
      core: { invoke: invokeMock },
      event: { listen: vi.fn(async () => vi.fn()) },
    };

    const api = mountPlanTaskSplit(container, { masterId: 'task_alpha' });
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        'set_binding',
        expect.objectContaining({ binding: expect.any(Object) }),
      );
    });

    invokeMock.mockClear();
    api.dispose();
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('reset_binding');
    });
    const resetCalls = invokeMock.mock.calls.filter((c) => c[0] === 'reset_binding');
    expect(resetCalls).toHaveLength(1);

    // Idempotent second dispose / unmount: must not re-issue Reset
    invokeMock.mockClear();
    api.dispose();
    api.unmount();
    await new Promise((r) => setTimeout(r, 30));
    expect(invokeMock).not.toHaveBeenCalledWith('reset_binding');

    const exec = await window.__TAURI__.core.invoke('execute_binding');
    expect(exec.ok).toBe(false);
    expect(exec.code).toBe('rejected_unbound');

    container.remove();
    delete window.__TAURI__;
  });

  it('shell_close is not cut acceptance on Todos leave path', () => {
    expect(planTaskIndexJs).not.toMatch(
      /shell_close[\s\S]{0,160}resetTodosBinding|resetTodosBinding[\s\S]{0,160}shell_close/i,
    );
    expect(planTaskIndexJs).not.toMatch(
      /shell_close[\s\S]{0,160}onTodosPageLeave|onTodosPageLeave[\s\S]{0,160}shell_close/i,
    );
  });
});
