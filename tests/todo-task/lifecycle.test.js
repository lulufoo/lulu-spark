// @vitest-environment jsdom
/**
 * t2: Todos page lifecycle keeps enter/leave exports; page path is zero Binding.
 * Sources: tech-doc C3-VF / C7_4a-T
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const getJsonMock = vi.fn();

vi.mock('../../frontend/src/host/apiClient.ts', async (importOriginal) => {
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
  mountTodoTaskSplit,
  onTodosPageEnter,
  onMasterSelectionChange,
  onTodosPageLeave,
} from '../../frontend/src/todo-task/index.ts';
import { readTodoTaskUiSource } from '../helpers/todo-task-ui-source.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const todoTaskIndexJs = readTodoTaskUiSource();
const todosLifecycleJs = readFileSync(
  join(fixtureRoot, 'frontend/src/todo-task/commands/lifecycle.ts'),
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

function expectNoBindingInvoke(invokeMock) {
  expect(invokeMock).not.toHaveBeenCalledWith(
    'set_binding',
    expect.anything(),
  );
  expect(invokeMock).not.toHaveBeenCalledWith('set_binding');
  expect(invokeMock).not.toHaveBeenCalledWith('reset_binding');
  expect(invokeMock).not.toHaveBeenCalledWith(
    'reset_binding',
    expect.anything(),
  );
}

describe('Todos page lifecycle — source contracts', () => {
  it('exports page lifecycle entry points from todo-task/index.js', () => {
    expect(typeof createTodosPageLifecycle).toBe('function');
    expect(typeof onTodosPageEnter).toBe('function');
    expect(typeof onMasterSelectionChange).toBe('function');
    expect(typeof onTodosPageLeave).toBe('function');
    expect(todoTaskIndexJs).toMatch(/onTodosPageEnter|createTodosPageLifecycle/);
    expect(todoTaskIndexJs).toMatch(/onMasterSelectionChange/);
    expect(todoTaskIndexJs).toMatch(/onTodosPageLeave/);
  });

  it('lifecycle internals no longer call Binding helpers', () => {
    expect(todosLifecycleJs).not.toMatch(/buildTodosBinding|resetTodosBinding/);
    expect(todosLifecycleJs).not.toMatch(/set_binding|reset_binding/);
    expect(todoTaskIndexJs).not.toMatch(/buildTodosBinding|resetTodosBinding/);
    expect(todoTaskIndexJs).toMatch(
      /function dispose\(\)\s*\{[\s\S]*?onTodosPageLeave/,
    );
    expect(todoTaskIndexJs).not.toMatch(
      /shell_close[\s\S]{0,120}onTodosPageLeave|onTodosPageLeave[\s\S]{0,120}shell_close/i,
    );
  });
});

describe('createTodosPageLifecycle — zero Binding', () => {
  let invokeMock;
  let events;

  beforeEach(() => {
    events = [];
    invokeMock = vi.fn(async (cmd, args) => {
      if (cmd === 'set_binding') {
        return { ok: true, state: 'bound', key: args?.binding?.key };
      }
      if (cmd === 'reset_binding') {
        return { ok: true, state: 'unbound' };
      }
      if (cmd === 'execute_binding') {
        return { ok: false, code: 'rejected_unbound', state: 'unbound' };
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

  it('enter with selected master does not Set', async () => {
    const life = createTodosPageLifecycle(trackCallbacks());
    await life.onTodosPageEnter('task_alpha');
    expectNoBindingInvoke(invokeMock);
    expect(events.filter((e) => e.event === 'onBound')).toHaveLength(0);
  });

  it('enter without a live instance does not Set; later selection does not Set', async () => {
    const life = createTodosPageLifecycle(trackCallbacks());
    await life.onTodosPageEnter();
    await life.onMasterSelectionChange('task_alpha');
    expectNoBindingInvoke(invokeMock);
    expect(events.map((e) => e.event)).toEqual([]);
  });

  it('in-page master change does not replace Set and does not Present', async () => {
    const life = createTodosPageLifecycle(trackCallbacks());
    await life.onTodosPageEnter('task_alpha');
    invokeMock.mockClear();
    await life.onMasterSelectionChange('task_beta');
    expectNoBindingInvoke(invokeMock);
    expect(invokeMock).not.toHaveBeenCalledWith(
      'open_ai_assistant',
      expect.anything(),
    );
    expect(invokeMock).not.toHaveBeenCalledWith(
      'present_ai_assistant',
      expect.anything(),
    );
  });

  it('leave / dispose does not Reset', async () => {
    const life = createTodosPageLifecycle(trackCallbacks());
    await life.onTodosPageEnter('task_alpha');
    invokeMock.mockClear();
    events.length = 0;
    await life.onTodosPageLeave();
    expectNoBindingInvoke(invokeMock);
    expect(events.filter((e) => e.event === 'onUnbound')).toHaveLength(0);
  });

  it('shell close ≠ Reset: no Host Reset and no onUnbound', async () => {
    const life = createTodosPageLifecycle(trackCallbacks());
    await life.onTodosPageEnter('task_alpha');
    events.length = 0;
    const close = life.notifyShellClose();
    expect(close.reset).toBe(false);
    expect(events.filter((e) => e.event === 'onUnbound')).toHaveLength(0);
    expect(invokeMock).not.toHaveBeenCalledWith('reset_binding');
  });
});

describe('mountTodoTaskSplit wires page lifecycle without Binding', () => {
  let container;
  let invokeMock;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    getJsonMock.mockReset();
    getJsonMock.mockResolvedValue(sampleMasters);

    invokeMock = vi.fn(async (cmd, args) => {
      if (cmd === 'set_binding') {
        return { ok: true, state: 'bound' };
      }
      if (cmd === 'reset_binding') {
        return { ok: true, state: 'unbound' };
      }
      if (cmd === 'open_ai_assistant') {
        return {
          session_id: 'sess_1',
          bound_master_task_id: args?.masterTaskId,
          window_label: 'ai-assistant',
          busy: false,
        };
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
  });

  afterEach(() => {
    container?.remove();
    delete window.__TAURI__;
    vi.restoreAllMocks();
  });

  it('mount / selection change / dispose issue zero Set/Reset', async () => {
    const api = mountTodoTaskSplit(container, { masterId: 'task_alpha' });
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-detail-toolbar')).not.toBeNull();
    });
    expectNoBindingInvoke(invokeMock);

    invokeMock.mockClear();
    const betaBtn = container.querySelector(
      '.todo-task-master-item[data-master-id="task_beta"]',
    );
    expect(betaBtn).toBeTruthy();
    betaBtn.click();
    await vi.waitFor(() => {
      expect(
        container.querySelector(
          '.todo-task-master-item--selected[data-master-id="task_beta"]',
        ),
      ).toBeTruthy();
    });
    expectNoBindingInvoke(invokeMock);
    expect(invokeMock).not.toHaveBeenCalledWith(
      'open_ai_assistant',
      expect.anything(),
    );

    invokeMock.mockClear();
    api.dispose();
    await new Promise((r) => setTimeout(r, 30));
    expectNoBindingInvoke(invokeMock);
  });

  it('mount without selection still chooses first displayed master and does not Set', async () => {
    const api = mountTodoTaskSplit(container, { masterId: '' });
    await vi.waitFor(() => {
      expect(
        container.querySelector('.todo-task-master-item--selected'),
      ).not.toBeNull();
    });
    const selected = container.querySelector(
      '.todo-task-master-item--selected',
    );
    expect(selected?.dataset.masterId).toBe('task_beta');
    expectNoBindingInvoke(invokeMock);
    api.dispose();
  });
});

describe('t2 Todos leave is not a Binding Reset', () => {
  it('leave wiring stays explicit but no longer Resets Binding', () => {
    expect(todoTaskIndexJs).toMatch(
      /function dispose\(\)\s*\{[\s\S]*?onTodosPageLeave/,
    );
    expect(todosLifecycleJs).not.toMatch(/resetTodosBinding|reset_binding/);
    expect(todoTaskIndexJs).not.toMatch(
      /shell_close[\s\S]{0,160}resetTodosBinding|resetTodosBinding[\s\S]{0,160}shell_close/i,
    );
    expect(todoTaskIndexJs).not.toMatch(
      /shell_close[\s\S]{0,160}onTodosPageLeave|onTodosPageLeave[\s\S]{0,160}shell_close/i,
    );
  });

  it('dispose / unmount is idempotent and never issues Reset', async () => {
    const container = document.createElement('div');
    document.body.appendChild(container);
    getJsonMock.mockReset();
    getJsonMock.mockResolvedValue(sampleMasters);
    const invokeMock = vi.fn(async (cmd) => {
      if (cmd === 'set_binding') return { ok: true, state: 'bound' };
      if (cmd === 'reset_binding') return { ok: true, state: 'unbound' };
      if (cmd === 'ensure_ai_assistant_session') {
        return { session_id: 'sess_1', busy: false };
      }
      return {};
    });
    window.__TAURI__ = {
      core: { invoke: invokeMock },
      event: { listen: vi.fn(async () => vi.fn()) },
    };

    const api = mountTodoTaskSplit(container, { masterId: 'task_alpha' });
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-detail-toolbar')).not.toBeNull();
    });

    invokeMock.mockClear();
    api.dispose();
    await new Promise((r) => setTimeout(r, 30));
    expectNoBindingInvoke(invokeMock);

    invokeMock.mockClear();
    api.dispose();
    api.unmount();
    await new Promise((r) => setTimeout(r, 30));
    expectNoBindingInvoke(invokeMock);

    container.remove();
    delete window.__TAURI__;
  });
});
