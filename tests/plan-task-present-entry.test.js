// @vitest-environment jsdom
/**
 * L2 t3: 页内 Present 入口——仅开窗/聚焦，停止打开并绑定.
 * Sources: tech-doc L06-T / L08-AR / L11-AR / L12-I / N1 / N2
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
const aiAssistantCmd = readFileSync(
  join(fixtureRoot, 'src-tauri/src/commands/ai_assistant.rs'),
  'utf8',
);
const libRs = readFileSync(join(fixtureRoot, 'src-tauri/src/lib.rs'), 'utf8');

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

describe('Todos Present entry — source contracts (t3)', () => {
  it('openPlanAiAssistant / presentTodosAssistant invokes Host Present, not open_ai_assistant bind path', () => {
    // Button handler must Present via present_ai_assistant (→ create_or_focus),
    // not invoke('open_ai_assistant', { masterTaskId }).
    expect(planTaskIndex).toMatch(
      /presentTodosAssistant|openPlanAiAssistant/,
    );
    expect(planTaskIndex).toMatch(/present_ai_assistant/);
    expect(planTaskIndex).not.toMatch(
      /invoke\(\s*['"]open_ai_assistant['"]\s*,\s*\{\s*masterTaskId/,
    );
  });

  it('Host Present maps to create_or_focus_ai_assistant_window and is registered', () => {
    expect(aiAssistantCmd).toMatch(/fn present_ai_assistant\b/);
    expect(aiAssistantCmd).toMatch(/create_or_focus_ai_assistant_window/);
    expect(libRs).toMatch(/present_ai_assistant/);
    expect(libRs).toMatch(/fn create_or_focus_ai_assistant_window\s*\(/);
  });

  it('does not upgrade FAB Top3 (plan-task-assistant.js) to chat Present or Set', () => {
    expect(planTaskAssistant).not.toMatch(/present_ai_assistant/);
    expect(planTaskAssistant).not.toMatch(/open_ai_assistant/);
    expect(planTaskAssistant).not.toMatch(/set_binding/);
    expect(planTaskAssistant).not.toMatch(/ai-assistant:turn-completed/);
  });
});

describe('mountPlanTaskSplit Present entry (t3)', () => {
  let container;
  let invokeMock;
  let hostBound;
  let hostBindingToken;
  let setCount;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    getJsonMock.mockReset();
    getJsonMock.mockResolvedValue(sampleMasters);
    hostBound = false;
    hostBindingToken = null;
    setCount = 0;

    invokeMock = vi.fn(async (cmd, args) => {
      if (cmd === 'set_binding') {
        const binding = args?.binding;
        if (
          !binding ||
          binding.tools == null ||
          binding.prompt == null ||
          binding.callbacks == null
        ) {
          return {
            ok: false,
            code: 'set_invalid',
            state: hostBound ? 'bound' : 'unbound',
          };
        }
        setCount += 1;
        hostBound = true;
        hostBindingToken = `tok_${setCount}`;
        return { ok: true, state: 'bound' };
      }
      if (cmd === 'reset_binding') {
        hostBound = false;
        hostBindingToken = null;
        return { ok: true, state: 'unbound' };
      }
      if (cmd === 'query_binding') {
        return { state: hostBound ? 'bound' : 'unbound' };
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
      if (cmd === 'present_ai_assistant') {
        return {
          surface: 'Present',
          window_label: 'ai-assistant',
        };
      }
      if (cmd === 'open_ai_assistant') {
        return {
          session_id: 'sess_legacy',
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

  it('Assistant button Present only: present_ai_assistant; no open_ai_assistant; no Set/bound write', async () => {
    const { dispose } = mountPlanTaskSplit(container, {
      masterId: 'task_alpha',
    });
    await vi.waitFor(() => {
      expect(
        container.querySelector('[data-action="open-ai-assistant"]'),
      ).not.toBeNull();
    });

    // Clear enter-path Set calls so Present click assertions are isolated.
    const setsBeforeClick = invokeMock.mock.calls.filter(
      ([cmd]) => cmd === 'set_binding',
    ).length;
    expect(setsBeforeClick).toBeGreaterThan(0);
    invokeMock.mockClear();

    container.querySelector('[data-action="open-ai-assistant"]').click();
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('present_ai_assistant');
    });

    expect(invokeMock).not.toHaveBeenCalledWith(
      'open_ai_assistant',
      expect.anything(),
    );
    expect(invokeMock).not.toHaveBeenCalledWith(
      'set_binding',
      expect.anything(),
    );
    // Present must not pass / write business bind key.
    const presentCalls = invokeMock.mock.calls.filter(
      ([cmd]) => cmd === 'present_ai_assistant',
    );
    expect(presentCalls.length).toBeGreaterThan(0);
    for (const call of presentCalls) {
      const args = call[1];
      if (args && typeof args === 'object') {
        expect(args).not.toHaveProperty('masterTaskId');
        expect(args).not.toHaveProperty('bound_master_task_id');
        expect(args).not.toHaveProperty('master_task_id');
      }
    }
    dispose();
  });

  it('Present keeps Binding state unchanged (query before/after)', async () => {
    const { dispose } = mountPlanTaskSplit(container, {
      masterId: 'task_alpha',
    });
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        'set_binding',
        expect.objectContaining({ binding: expect.any(Object) }),
      );
    });

    const before = await window.__TAURI__.core.invoke('query_binding');
    expect(before.state).toBe('bound');
    const tokenBefore = hostBindingToken;

    invokeMock.mockClear();
    container.querySelector('[data-action="open-ai-assistant"]').click();
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('present_ai_assistant');
    });

    const after = await window.__TAURI__.core.invoke('query_binding');
    expect(after.state).toBe('bound');
    expect(hostBound).toBe(true);
    expect(hostBindingToken).toBe(tokenBefore);
    expect(invokeMock).not.toHaveBeenCalledWith('reset_binding');
    expect(invokeMock).not.toHaveBeenCalledWith(
      'set_binding',
      expect.anything(),
    );
    dispose();
  });

  it('N2: without successful Set, Present opens shell but execute is rejected', async () => {
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'set_binding') {
        return { ok: false, code: 'set_invalid', state: 'unbound' };
      }
      if (cmd === 'present_ai_assistant') {
        return { surface: 'Present', window_label: 'ai-assistant' };
      }
      if (cmd === 'query_binding') {
        return { state: 'unbound' };
      }
      if (cmd === 'execute_binding') {
        return { ok: false, code: 'rejected_unbound', state: 'unbound' };
      }
      if (cmd === 'reset_binding') {
        return { ok: true, state: 'unbound' };
      }
      return {};
    });

    const { dispose } = mountPlanTaskSplit(container, {
      masterId: 'task_alpha',
    });
    await vi.waitFor(() => {
      expect(
        container.querySelector('[data-action="open-ai-assistant"]'),
      ).not.toBeNull();
    });

    container.querySelector('[data-action="open-ai-assistant"]').click();
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('present_ai_assistant');
    });

    const q = await window.__TAURI__.core.invoke('query_binding');
    expect(q.state).toBe('unbound');
    const exec = await window.__TAURI__.core.invoke('execute_binding');
    expect(exec.ok).toBe(false);
    expect(exec.code).toBe('rejected_unbound');
    expect(invokeMock).not.toHaveBeenCalledWith(
      'open_ai_assistant',
      expect.anything(),
    );
    dispose();
  });
});
