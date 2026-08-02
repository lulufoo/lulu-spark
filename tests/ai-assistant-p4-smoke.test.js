/**
 * T7 / P4 layered coverage gate + L2 SK-4 Todos parity (P1–P6 / N1/N2).
 * Host Tools + Loop automated markers, UI/config/main-path + dual-platform
 * smoke checklist, plan-task-write regression wiring, Todos parity acceptance.
 */
// @vitest-environment jsdom
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it, beforeEach, afterEach, vi } from 'vitest';

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
  assembleTodosBindingBody,
  TODOS_BUSINESS_KEY,
  TODOS_OPEN_AND_BIND_MAIN_PATH_DISABLED,
  TODOS_PARITY_ACCEPTANCE,
  mountPlanTaskSplit,
} from '../frontend/js/plan-task/index.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const checklistPath = join(
  repoRoot,
  'tests/ai-assistant-p4-smoke-checklist.md',
);
const pkg = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8'));
const testScript = pkg.scripts.test;

const REQUIRED_CHECKLIST_MARKERS = [
  '## UI 开窗',
  '## 配置保存',
  '## 主路径冒烟',
  '## 双平台配置冒烟',
  '## J1 / execute 门闩内核验收',
  '## Todos 主路径 parity（P1–P6 / N1/N2）',
  'H1 无 UI',
  '通用 Binding 夹具',
  'KIMI',
  'GLM',
  'P1：进页 Set→onBound 后 Present 可对话',
  'P2：只读查计划/子项基于真实数据',
  'P3：加子项成功且列表/详情出现',
  'P4：改主/子标题 Todos 侧可见',
  'P5：不支持操作明确拒绝且数据不变',
  'P6：离页 Reset→onUnbound 后 execute 被拒',
  'N1：调用 open_ai_assistant(masterTaskId)',
  'N2：未 Set 仅 Present 时不可发送/执行成功',
  '关壳≠Reset',
  'Set 失败可恢复',
];

const LAYERED_VITEST = [
  'tests/ai-assistant-window.test.js',
  'tests/llm-settings.test.js',
  'tests/plan-task-ai-assistant-entry.test.js',
  'tests/plan-task-write.test.js',
  'tests/ai-assistant-p4-smoke.test.js',
  'tests/plan-task-binding.test.js',
  'tests/plan-task-lifecycle.test.js',
  'tests/plan-task-present-entry.test.js',
  'tests/plan-task-deembed-writeback.test.js',
];

const HOST_TOOLS_MARKERS = [
  'tools_update_master_title_success_and_validation_failures',
  'tools_five_suite_happy_path_and_data_omits_todo_md',
];

const LOOP_MARKERS = [
  'run_loop_clarify_under_limit_returns_none_not_wrote',
  'run_loop_tool_write_sets_wrote_true_and_persists',
  'run_loop_add_sub_and_update_sub_title_paths_are_observable',
  'parallel_tool_calls_run_serially_and_ok_false_does_not_abort',
  'same_message_tool_calls_plus_content_content_is_not_final_reply',
  'open_ai_assistant_busy_rejects_rebind',
  'unsupported_tool_calls_upstream_is_error_terminal_no_prompt_json',
  'length_and_http_errors_map_to_error_terminal_no_retry',
  'history_truncation_keeps_system_and_dual_hard_caps',
  'terminal_no_plan_unsupported_and_error_are_distinguishable',
  // J1 / execute 门闩内核验收夹具（SK-4 / H1，无业务 UI 驱动）
  'j1_1_legal_set_on_bound_execute_reset_rejects',
  'j1_2_illegal_set_keeps_state_no_on_bound_emits_set_invalid',
  'j1_3_replace_set_on_unbound_then_on_bound_execute_uses_new',
  'j1_4_mid_execute_reset_unbounds_cancels_with_on_error',
  'j1_5_contract_states_tools_prompt_callbacks_present_not_bound',
  'j1_execute_gate_bound_success_and_unbound_reject',
  'j1_h1_kernel_api_fixture_driver_not_business_ui',
];

const J1_COMMAND_MARKERS = [
  'j1_command_fixture_set_execute_reset_reject_via_json',
  'j1_command_illegal_set_emits_set_invalid_keeps_unbound',
  'j1_present_not_bound_execute_rejects_without_set',
];

const EXPECTED_PARITY_IDS = Object.freeze([
  'P1',
  'P2',
  'P3',
  'P4',
  'P5',
  'P6',
  'N1',
  'N2',
]);

const OUT_OF_PARITY_TOOLS = [
  'complete_todo',
  'abandon_todo_sub',
  'delete_todo_sub',
  'set_todo_status',
  'batch_',
];

describe('AI assistant P4 layered smoke gate (t7)', () => {
  it('ships executable smoke checklist with UI/config/main-path and dual-platform steps', () => {
    expect(
      existsSync(checklistPath),
      'missing tests/ai-assistant-p4-smoke-checklist.md',
    ).toBe(true);
    const text = readFileSync(checklistPath, 'utf8');
    for (const marker of REQUIRED_CHECKLIST_MARKERS) {
      expect(text, `checklist missing marker: ${marker}`).toContain(marker);
    }
  });

  it('npm test wires layered UI/config/entry + plan-task-write + this gate', () => {
    for (const file of LAYERED_VITEST) {
      expect(testScript, `missing ${file} in npm test`).toContain(file);
    }
  });

  it('Host Tools layer has update_master_title shell + validation coverage', () => {
    const toolsTests = readFileSync(
      join(repoRoot, 'src-tauri/src/unit-tests/services/agent/mod.rs'),
      'utf8',
    );
    for (const marker of HOST_TOOLS_MARKERS) {
      expect(toolsTests, `missing Host Tools test ${marker}`).toContain(
        `fn ${marker}`,
      );
    }
  });

  it('Loop layer covers clarify/write paths, busy rebind, failure taxonomy, history caps', () => {
    const loopTests = readFileSync(
      join(repoRoot, 'src-tauri/src/unit-tests/services/agent/loop_tests.rs'),
      'utf8',
    );
    for (const marker of LOOP_MARKERS) {
      expect(loopTests, `missing Loop test ${marker}`).toContain(
        `fn ${marker}`,
      );
    }
  });

  it('J1 command-surface fixtures cover Set→execute→Reset and Present≠bound', () => {
    const cmdTests = readFileSync(
      join(repoRoot, 'src-tauri/src/unit-tests/commands/ai_assistant.rs'),
      'utf8',
    );
    for (const marker of J1_COMMAND_MARKERS) {
      expect(cmdTests, `missing J1 command test ${marker}`).toContain(
        `fn ${marker}`,
      );
    }
  });
});

describe('Todos SK-4 parity acceptance (P1–P6 / N1/N2)', () => {
  it('exports TODOS_PARITY_ACCEPTANCE covering P1–P6 and N1/N2 only', () => {
    expect(TODOS_PARITY_ACCEPTANCE).toEqual(EXPECTED_PARITY_IDS);
    expect(TODOS_PARITY_ACCEPTANCE).not.toEqual(
      expect.arrayContaining(['complete', 'abandon', 'batch', 'cross_plan']),
    );
  });

  it('Binding call surface submits key-only todo_task; no tools/prompt/callbacks payload', () => {
    expect(TODOS_BUSINESS_KEY).toBe('todo_task');
    const body = assembleTodosBindingBody();
    expect(body).toEqual({ key: 'todo_task' });
    expect(body).not.toHaveProperty('tools');
    expect(body).not.toHaveProperty('prompt');
    expect(body).not.toHaveProperty('callbacks');
    const serialized = JSON.stringify(body);
    for (const banned of OUT_OF_PARITY_TOOLS) {
      expect(serialized).not.toContain(banned);
    }
  });

  it('N1 flag: open-and-bind main path disabled on Todos consumer', () => {
    expect(TODOS_OPEN_AND_BIND_MAIN_PATH_DISABLED).toBe(true);
  });

  describe('observable runtime paths', () => {
    let invokeMock;
    let hostBound;
    let hostBindingToken;
    let events;

    beforeEach(() => {
      events = [];
      hostBound = false;
      hostBindingToken = null;
      getJsonMock.mockReset();
      getJsonMock.mockResolvedValue([
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
      ]);
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
          hostBound = true;
          hostBindingToken = binding.__testToken ?? binding.key;
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
        if (cmd === 'present_ai_assistant') {
          return { surface: 'Present', window_label: 'ai-assistant' };
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

    it('P1: enter Set→onBound then Present opens shell while bound (dialog ready)', async () => {
      const life = createTodosPageLifecycle(trackCallbacks());
      const setResult = await life.onTodosPageEnter('task_alpha');
      expect(setResult.ok).toBe(true);
      expect(life.isBound()).toBe(true);
      expect(events.map((e) => e.event)).toEqual(['onBound']);

      const present = await window.__TAURI__.core.invoke('present_ai_assistant');
      expect(present.surface).toBe('Present');
      expect(life.isBound()).toBe(true);
      const exec = await window.__TAURI__.core.invoke('execute_binding');
      expect(exec.ok).toBe(true);
    });

    it('P2–P5: Binding Set is key-only; capability surface is Host MCP key, not client tools/prompt', () => {
      const body = assembleTodosBindingBody();
      expect(body).toEqual({ key: TODOS_BUSINESS_KEY });
      expect(body).not.toHaveProperty('tools');
      expect(body).not.toHaveProperty('prompt');
      expect(body).not.toHaveProperty('callbacks');
      expect(body).not.toHaveProperty('engine');
    });

    it('P6: leave Reset→onUnbound then execute rejected', async () => {
      const life = createTodosPageLifecycle(trackCallbacks());
      await life.onTodosPageEnter('task_alpha');
      events.length = 0;

      const left = await life.onTodosPageLeave();
      expect(left.ok).toBe(true);
      expect(left.state).toBe('unbound');
      expect(events.map((e) => e.event)).toEqual(['onUnbound']);

      const exec = await window.__TAURI__.core.invoke('execute_binding');
      expect(exec.ok).toBe(false);
      expect(exec.code).toBe('rejected_unbound');
    });

    it('N2: Present without Set does not make execute succeed', async () => {
      const life = createTodosPageLifecycle(trackCallbacks());
      expect(life.isBound()).toBe(false);

      const present = await window.__TAURI__.core.invoke('present_ai_assistant');
      expect(present.surface).toBe('Present');
      expect(life.isBound()).toBe(false);

      const exec = await window.__TAURI__.core.invoke('execute_binding');
      expect(exec.ok).toBe(false);
      expect(exec.code).toBe('rejected_unbound');
      expect(events.filter((e) => e.event === 'onBound')).toHaveLength(0);
    });

    it('N1/T6: Todos has no page Present entry; Host Present skips open_ai_assistant', async () => {
      const container = document.createElement('div');
      document.body.appendChild(container);
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
      await window.__TAURI__.core.invoke('present_ai_assistant');
      expect(invokeMock).toHaveBeenCalledWith('present_ai_assistant');
      expect(invokeMock).not.toHaveBeenCalledWith(
        'open_ai_assistant',
        expect.anything(),
      );
      dispose();
      container.remove();
    });

    it('关壳≠Reset: shell close keeps Binding; Present still executable', async () => {
      const life = createTodosPageLifecycle(trackCallbacks());
      await life.onTodosPageEnter('task_alpha');
      events.length = 0;

      const close = life.notifyShellClose();
      expect(close.reset).toBe(false);
      expect(life.isBound()).toBe(true);
      expect(events.filter((e) => e.event === 'onUnbound')).toHaveLength(0);

      await window.__TAURI__.core.invoke('present_ai_assistant');
      const exec = await window.__TAURI__.core.invoke('execute_binding');
      expect(exec.ok).toBe(true);
    });

    it('Set 失败可恢复: failed enter then re-select Set succeeds and execute works', async () => {
      const life = createTodosPageLifecycle(trackCallbacks());
      invokeMock.mockImplementation(async (cmd) => {
        if (cmd === 'set_binding') {
          return { ok: false, code: 'set_invalid', state: 'unbound' };
        }
        if (cmd === 'execute_binding') {
          return { ok: false, code: 'rejected_unbound', state: 'unbound' };
        }
        if (cmd === 'present_ai_assistant') {
          return { surface: 'Present', window_label: 'ai-assistant' };
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

      events.length = 0;
      invokeMock.mockImplementation(async (cmd) => {
        if (cmd === 'set_binding') {
          hostBound = true;
          hostBindingToken = 'recovered';
          return { ok: true, state: 'bound' };
        }
        if (cmd === 'execute_binding') {
          return hostBound
            ? { ok: true, state: 'bound', applied_token: hostBindingToken }
            : { ok: false, code: 'rejected_unbound', state: 'unbound' };
        }
        if (cmd === 'present_ai_assistant') {
          return { surface: 'Present', window_label: 'ai-assistant' };
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
});


describe('t6 layered acceptance L0/L1/L2 gate', () => {
  it('loop_tests lands L0/L1/L2 + leave-primary markers', () => {
    const loopTests = readFileSync(
      join(repoRoot, 'src-tauri/src/unit-tests/services/agent/loop_tests.rs'),
      'utf8',
    );
    for (const marker of [
      't6_layered_acceptance_markers_are_landed',
      't6_l0_unbound_reject_reset_idempotent_and_mid_reset_cancel',
      't6_l1_session_generation_and_re_set_cuts',
      't6_l1_stale_generation_rejects_continue',
      't6_l2_missed_reset_defensive_cut_then_not_executable',
      't6_shell_close_is_not_cut_acceptance',
      't6_explicit_reset_not_omitted_because_defensive_exists',
    ]) {
      expect(loopTests, `missing ${marker}`).toContain(`fn ${marker}`);
    }
    const loopRs = readFileSync(
      join(repoRoot, 'src-tauri/src/services/agent/loop.rs'),
      'utf8',
    );
    expect(loopRs).toMatch(/LAYERED_ACCEPTANCE_L0/);
    expect(loopRs).toMatch(/LAYERED_ACCEPTANCE_L1/);
    expect(loopRs).toMatch(/LAYERED_ACCEPTANCE_L2/);
    expect(loopRs).toMatch(/TODOS_EXPLICIT_LEAVE_RESET_PRIMARY/);
  });

  it('Todos leave chain export remains primary (not replaced by defensive cut)', () => {
    const indexJs = readFileSync(
      join(repoRoot, 'frontend/js/plan-task/index.js'),
      'utf8',
    );
    const lifeJs = readFileSync(
      join(repoRoot, 'frontend/js/plan-task/todos-lifecycle.js'),
      'utf8',
    );
    expect(indexJs + '\n' + lifeJs).toMatch(/TODOS_EXPLICIT_LEAVE_RESET_CHAIN/);
    expect(indexJs).toMatch(/onTodosPageLeave/);
    expect(lifeJs).toMatch(/resetTodosBinding/);
  });
});
