// @vitest-environment jsdom
/**
 * T6 / L18-T / L22-VF Todos: remove in-page Assistant entry; keep Set/Reset.
 * Present remains Host/corner (t3); page must not expose open-ai-assistant.
 * Sources: tech-doc T6 / L18-T / L13-I / L22-VF Todos
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

import { mountTodoTaskSplit } from '../../frontend/src/todo-task/index.ts';
import { readTodoTaskUiSource } from '../helpers/todo-task-ui-source.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const todoTaskIndex = readTodoTaskUiSource();
const todoTaskAssistant = readFileSync(
  join(fixtureRoot, 'frontend/src/todo-task/ui/assistant.tsx'),
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

describe('Todos page Assistant entry removed — source contracts (t6)', () => {
  it('removes presentTodosAssistant / open-ai-assistant page trigger paths', () => {
    expect(todoTaskIndex).not.toMatch(/presentTodosAssistant/);
    expect(todoTaskIndex).not.toMatch(/openPlanAiAssistant/);
    expect(todoTaskIndex).not.toMatch(/data-action=["']open-ai-assistant["']/);
    expect(todoTaskIndex).not.toMatch(
      /action\s*===\s*['"]open-ai-assistant['"]/,
    );
    expect(todoTaskIndex).not.toMatch(
      /invoke\(\s*['"]open_ai_assistant['"]\s*,\s*\{\s*masterTaskId/,
    );
  });

  it('keeps enter/leave lifecycle wiring without page Binding Set/Reset', () => {
    expect(todoTaskIndex).toMatch(/onTodosPageEnter|createTodosPageLifecycle/);
    expect(todoTaskIndex).toMatch(/onTodosPageLeave/);
    expect(todoTaskIndex).toMatch(
      /function dispose\(\)\s*\{[\s\S]*?onTodosPageLeave/,
    );
    expect(todoTaskIndex).not.toMatch(/buildTodosBinding|resetTodosBinding/);
  });

  it('Host Present remains available for corner/shell (t3); not page-owned', () => {
    expect(aiAssistantCmd).toMatch(/fn present_ai_assistant\b/);
    expect(aiAssistantCmd).toMatch(/entry_id/);
    const presentFn = aiAssistantCmd.match(
      /pub async fn present_ai_assistant[\s\S]*?^}/m,
    )?.[0];
    expect(presentFn, 'present_ai_assistant body').toBeTruthy();
    expect(presentFn).not.toMatch(/create_or_focus_ai_assistant_window/);
    expect(libRs).toMatch(/present_ai_assistant/);
  });

  // T7 / L22-VF Todos: page has no Assistant; Set/Reset remain the Binding main path.
  it('T7 VF: no in-page Assistant chrome; Present stays Host-owned shell path', () => {
    expect(todoTaskIndex).not.toMatch(/presentTodosAssistant/);
    expect(todoTaskIndex).not.toMatch(/data-action=["']open-ai-assistant["']/);
    expect(todoTaskIndex).toMatch(/onTodosPageEnter|onTodosPageLeave/);
    // Migrated: page must not invoke create_or_focus / open_ai_assistant as a live path.
    expect(todoTaskIndex).not.toMatch(/create_or_focus_ai_assistant_window/);
    expect(todoTaskIndex).not.toMatch(
      /invoke\(\s*['"]open_ai_assistant['"]/,
    );
  });

  it('does not upgrade FAB Top3 (todo-task-assistant.js) to chat Present or Set', () => {
    expect(todoTaskAssistant).not.toMatch(/present_ai_assistant/);
    expect(todoTaskAssistant).not.toMatch(/open_ai_assistant/);
    expect(todoTaskAssistant).not.toMatch(/set_binding/);
    expect(todoTaskAssistant).not.toMatch(/ai-assistant:turn-completed/);
  });
});

describe('mountTodoTaskSplit — no page Assistant; zero page Set/Reset (t2)', () => {
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
          typeof binding.key !== 'string' ||
          !binding.key.trim() ||
          binding.tools != null ||
          binding.prompt != null ||
          binding.callbacks != null
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

  it('renders no open-ai-assistant button when a master is selected', async () => {
    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-detail-toolbar')).not.toBeNull();
    });
    expect(
      container.querySelector('[data-action="open-ai-assistant"]'),
    ).toBeNull();
    expect(container.textContent).not.toMatch(/\bAssistant\b/);
    dispose();
  });

  it('default Todos entry selects the first displayed active todo and does not Set', async () => {
    const { dispose } = mountTodoTaskSplit(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-master-item--selected')).not.toBeNull();
    });

    const selected = container.querySelector('.todo-task-master-item--selected');
    expect(selected?.dataset.masterId).toBe('task_alpha');
    expect(invokeMock).not.toHaveBeenCalledWith(
      'set_binding',
      expect.anything(),
    );
    expect(hostBound).toBe(false);
    dispose();
  });

  it('enter does not Set Binding; leave/dispose does not Reset', async () => {
    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-detail-toolbar')).not.toBeNull();
    });
    expect(invokeMock).not.toHaveBeenCalledWith(
      'set_binding',
      expect.anything(),
    );
    expect(hostBound).toBe(false);

    invokeMock.mockClear();
    dispose();
    await new Promise((r) => setTimeout(r, 30));
    expect(invokeMock).not.toHaveBeenCalledWith('reset_binding');
    expect(hostBound).toBe(false);
  });

  it('Present≠Set: Host Present does not write Binding; page has no Present trigger', async () => {
    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-detail-toolbar')).not.toBeNull();
    });

    const before = await window.__TAURI__.core.invoke('query_binding');
    expect(before.state).toBe('unbound');
    const tokenBefore = hostBindingToken;

    expect(
      container.querySelector('[data-action="open-ai-assistant"]'),
    ).toBeNull();

    invokeMock.mockClear();
    const present = await window.__TAURI__.core.invoke('present_ai_assistant');
    expect(present.surface).toBe('Present');

    const after = await window.__TAURI__.core.invoke('query_binding');
    expect(after.state).toBe('unbound');
    expect(hostBound).toBe(false);
    expect(hostBindingToken).toBe(tokenBefore);
    expect(invokeMock).not.toHaveBeenCalledWith('reset_binding');
    expect(invokeMock).not.toHaveBeenCalledWith(
      'set_binding',
      expect.anything(),
    );
    expect(invokeMock).not.toHaveBeenCalledWith(
      'open_ai_assistant',
      expect.anything(),
    );
    dispose();
  });

  it('N2: without successful Set, Host Present does not make execute succeed', async () => {
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

    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-detail-toolbar')).not.toBeNull();
    });
    expect(
      container.querySelector('[data-action="open-ai-assistant"]'),
    ).toBeNull();

    await window.__TAURI__.core.invoke('present_ai_assistant');
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
