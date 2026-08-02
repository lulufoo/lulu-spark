// @vitest-environment jsdom
/**
 * T6: Todos page has no in-page Assistant Present entry.
 * Keeps turn-completed writeback refresh; Binding Set is via page lifecycle.
 * Present = Host/corner present_ai_assistant only; not page open-ai-assistant.
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

describe('plan-task AI assistant entry source wiring (t6)', () => {
  it('plan-task/index.js has no page Present trigger; keeps turn-completed listen', () => {
    expect(planTaskIndex).toMatch(/ai-assistant:turn-completed/);
    expect(planTaskIndex).not.toMatch(/presentTodosAssistant/);
    expect(planTaskIndex).not.toMatch(/data-action=["']open-ai-assistant["']/);
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

  it('never shows open-ai-assistant entry (selected or not)', async () => {
    const { dispose } = mountPlanTaskSplit(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-master-item')).not.toBeNull();
    });
    expect(container.querySelector('[data-action="open-ai-assistant"]')).toBeNull();

    container.querySelector('[data-master-id="task_alpha"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-detail-toolbar')).not.toBeNull();
    });
    expect(container.querySelector('[data-action="open-ai-assistant"]')).toBeNull();
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
