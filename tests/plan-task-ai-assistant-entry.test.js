// @vitest-environment jsdom
/**
 * T6 / L2 t3: plan-page thin shell — Present entry + turn-completed refresh.
 * Does not cover FAB plan-task-assistant.js.
 * Present = present_ai_assistant only; not open_ai_assistant(masterTaskId).
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

import { mountPlanTaskSplit } from '../frontend/js/plan-task/index.js';

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

describe('plan-task AI assistant entry source wiring (t6 / t3)', () => {
  it('plan-task/index.js invokes present_ai_assistant and listens turn-completed', () => {
    expect(planTaskIndex).toMatch(/present_ai_assistant/);
    expect(planTaskIndex).toMatch(/ai-assistant:turn-completed/);
    expect(planTaskIndex).not.toMatch(
      /invoke\(\s*['"]open_ai_assistant['"]\s*,\s*\{\s*masterTaskId/,
    );
  });

  it('does not move Present / open_ai_assistant into plan-task-assistant FAB', () => {
    expect(planTaskAssistant).not.toMatch(/open_ai_assistant/);
    expect(planTaskAssistant).not.toMatch(/present_ai_assistant/);
    expect(planTaskAssistant).not.toMatch(/ai-assistant:turn-completed/);
  });
});

describe('mountPlanTaskSplit AI assistant entry', () => {
  let container;
  let invokeMock;
  let listenMock;
  let unlistenMock;
  let turnCompletedHandler;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    getJsonMock.mockReset();
    getJsonMock.mockResolvedValue(sampleMasters);

    invokeMock = vi.fn().mockImplementation(async (cmd) => {
      if (cmd === 'present_ai_assistant') {
        return { surface: 'Present', window_label: 'ai-assistant' };
      }
      if (cmd === 'set_binding') {
        return { ok: true, state: 'bound' };
      }
      if (cmd === 'reset_binding') {
        return { ok: true, state: 'unbound' };
      }
      return {};
    });
    unlistenMock = vi.fn();
    turnCompletedHandler = null;
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
    container.remove();
    delete window.__TAURI__;
  });

  it('shows open-ai-assistant entry only when a master is selected', async () => {
    const { dispose } = mountPlanTaskSplit(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-master-item')).not.toBeNull();
    });
    expect(container.querySelector('[data-action="open-ai-assistant"]')).toBeNull();

    container.querySelector('[data-master-id="task_alpha"]').click();
    await vi.waitFor(() => {
      expect(
        container.querySelector('[data-action="open-ai-assistant"]'),
      ).not.toBeNull();
    });
    dispose();
  });

  it('invokes present_ai_assistant without masterTaskId bind args', async () => {
    const { dispose } = mountPlanTaskSplit(container, {
      masterId: 'task_alpha',
    });
    await vi.waitFor(() => {
      expect(
        container.querySelector('[data-action="open-ai-assistant"]'),
      ).not.toBeNull();
    });

    invokeMock.mockClear();
    container.querySelector('[data-action="open-ai-assistant"]').click();
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('present_ai_assistant');
    });
    expect(invokeMock).not.toHaveBeenCalledWith(
      'open_ai_assistant',
      expect.anything(),
    );
    dispose();
  });

  it('listens ai-assistant:turn-completed and refreshes when wrote=true', async () => {
    const { dispose } = mountPlanTaskSplit(container, {
      masterId: 'task_alpha',
    });
    await vi.waitFor(() => {
      expect(listenMock).toHaveBeenCalledWith(
        'ai-assistant:turn-completed',
        expect.any(Function),
      );
    });

    const callsBefore = getJsonMock.mock.calls.length;
    expect(typeof turnCompletedHandler).toBe('function');
    await turnCompletedHandler({ payload: { wrote: true } });
    await vi.waitFor(() => {
      expect(getJsonMock.mock.calls.length).toBeGreaterThan(callsBefore);
    });
    dispose();
    expect(unlistenMock).toHaveBeenCalled();
  });

  it('does not refresh when wrote=false', async () => {
    const { dispose } = mountPlanTaskSplit(container, {
      masterId: 'task_alpha',
    });
    await vi.waitFor(() => {
      expect(typeof turnCompletedHandler).toBe('function');
    });

    const callsBefore = getJsonMock.mock.calls.length;
    await turnCompletedHandler({ payload: { wrote: false } });
    // Allow a tick; wrote=false must not schedule another list fetch.
    await Promise.resolve();
    expect(getJsonMock.mock.calls.length).toBe(callsBefore);
    dispose();
  });
});
