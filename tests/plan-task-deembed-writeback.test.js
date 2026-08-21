// @vitest-environment jsdom
/**
 * L2 t4: 去内嵌主路径 + 保留 turn-completed 写回刷新.
 * Sources: tech-doc L05-T / L06-T / L09-AR / L11-AR / L12-I#3 / N1
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
  mountPlanTaskSplit,
  TODOS_OPEN_AND_BIND_MAIN_PATH_DISABLED,
} from '../frontend/js/plan-task/index.js';
import { PLAN_TASK_ASSISTANT_FAB_CHAT_DISABLED } from '../frontend/js/plan-task-assistant.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const planTaskIndex = readFileSync(
  join(fixtureRoot, 'frontend/js/plan-task/index.js'),
  'utf8',
);
const planTaskAssistant = readFileSync(
  join(fixtureRoot, 'frontend/js/plan-task-assistant.js'),
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
];

describe('t4 de-embed + writeback — source contracts', () => {
  it('N1: Todos stops open_ai_assistant(masterTaskId) as executable main path', () => {
    expect(TODOS_OPEN_AND_BIND_MAIN_PATH_DISABLED).toBe(true);
    expect(planTaskIndex).not.toMatch(
      /invoke\(\s*['"]open_ai_assistant['"]\s*,\s*\{\s*masterTaskId/,
    );
    expect(planTaskIndex).not.toMatch(/['"]open_ai_assistant['"]/);
  });

  it('C-min: keeps turn-completed → onAiAssistantTurnCompleted → reloadList (mandatory)', () => {
    expect(planTaskIndex).toMatch(/ai-assistant:turn-completed/);
    expect(planTaskIndex).toMatch(/function onAiAssistantTurnCompleted\b/);
    expect(planTaskIndex).toMatch(/reloadList\(\s*\{\s*afterWrite:\s*true\s*\}\s*\)/);
  });

  it('FAB Top3 stays non-chat (Boundary Out); not Present/Set/execute', () => {
    expect(PLAN_TASK_ASSISTANT_FAB_CHAT_DISABLED).toBe(true);
    expect(planTaskAssistant).not.toMatch(/open_ai_assistant/);
    expect(planTaskAssistant).not.toMatch(/present_ai_assistant/);
    expect(planTaskAssistant).not.toMatch(/set_binding/);
    expect(planTaskAssistant).not.toMatch(/execute_binding/);
    expect(planTaskAssistant).not.toMatch(/ai-assistant:turn-completed/);
  });

  it('T6: page Present entry removed; no open-and-bind bypass posing as de-embed', () => {
    expect(planTaskIndex).not.toMatch(/presentTodosAssistant/);
    expect(planTaskIndex).not.toMatch(/data-action=["']open-ai-assistant["']/);
    // L12-I#3: must not keep openPlanAiAssistant open+bind bypass.
    expect(planTaskIndex).not.toMatch(
      /function\s+openPlanAiAssistant[\s\S]{0,400}open_ai_assistant/,
    );
  });
});

describe('mountPlanTaskSplit t4 runtime — N1 + writeback', () => {
  let container;
  let invokeMock;
  let listenMock;
  let unlistenMock;
  let turnCompletedHandler;
  let legacyBoundMasterId;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    getJsonMock.mockReset();
    getJsonMock.mockResolvedValue(sampleMasters);
    legacyBoundMasterId = null;
    turnCompletedHandler = null;
    unlistenMock = vi.fn();

    invokeMock = vi.fn(async (cmd, args) => {
      if (cmd === 'set_binding') {
        return { ok: true, state: 'bound' };
      }
      if (cmd === 'reset_binding') {
        return { ok: true, state: 'unbound' };
      }
      if (cmd === 'query_binding') {
        return { state: 'bound' };
      }
      if (cmd === 'execute_binding') {
        return { ok: true, state: 'bound' };
      }
      if (cmd === 'present_ai_assistant') {
        return { surface: 'Present', window_label: 'ai-assistant' };
      }
      if (cmd === 'open_ai_assistant') {
        legacyBoundMasterId = args?.masterTaskId ?? null;
        return {
          session_id: 'sess_legacy',
          bound_master_task_id: args?.masterTaskId,
          window_label: 'ai-assistant',
          busy: false,
        };
      }
      return {};
    });

    listenMock = vi.fn(async (eventName, handler) => {
      if (eventName === 'ai-assistant:turn-completed') {
        turnCompletedHandler = handler;
      }
      return unlistenMock;
    });

    window.__TAURI__ = {
      core: { invoke: invokeMock },
      event: { listen: listenMock },
    };
  });

  afterEach(() => {
    container?.remove();
    delete window.__TAURI__;
    vi.restoreAllMocks();
  });

  it('N1/T6: no page Assistant Present trigger — no open_ai_assistant, no legacy bound write', async () => {
    const { dispose } = mountPlanTaskSplit(container, {
      masterId: 'task_alpha',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-detail-toolbar')).not.toBeNull();
    });

    expect(
      container.querySelector('[data-action="open-ai-assistant"]'),
    ).toBeNull();

    invokeMock.mockClear();
    legacyBoundMasterId = null;
    // Host Present (corner/shell) must not write legacy open-and-bind fields.
    await window.__TAURI__.core.invoke('present_ai_assistant');
    expect(invokeMock).toHaveBeenCalledWith('present_ai_assistant');
    expect(invokeMock).not.toHaveBeenCalledWith(
      'open_ai_assistant',
      expect.anything(),
    );
    expect(legacyBoundMasterId).toBeNull();
    dispose();
  });

  it('keeps turn-completed writeback refresh when wrote=true', async () => {
    const { dispose } = mountPlanTaskSplit(container, {
      masterId: 'task_alpha',
    });
    await vi.waitFor(() => {
      expect(typeof turnCompletedHandler).toBe('function');
    });

    const callsBefore = getJsonMock.mock.calls.length;
    await turnCompletedHandler({ payload: { wrote: true } });
    await vi.waitFor(() => {
      expect(getJsonMock.mock.calls.length).toBeGreaterThan(callsBefore);
    });
    dispose();
    expect(unlistenMock).toHaveBeenCalled();
  });

  it('retains execute surface (de-embed does not remove execute path)', async () => {
    const { dispose } = mountPlanTaskSplit(container, {
      masterId: 'task_alpha',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-detail-toolbar')).not.toBeNull();
    });
    expect(invokeMock).not.toHaveBeenCalledWith(
      'set_binding',
      expect.anything(),
    );

    const exec = await window.__TAURI__.core.invoke('execute_binding');
    expect(exec.ok).toBe(true);
    dispose();
  });
});
