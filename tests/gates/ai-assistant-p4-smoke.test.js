/**
 * AI assistant layered smoke gate (post todo desktop removal).
 * Host Tools + Loop markers, binding surface, checklist wiring.
 */
// @vitest-environment jsdom
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it, beforeEach, afterEach, vi } from 'vitest';

import {
  setWorkbenchBinding,
  WORKBENCH_BUSINESS_KEY,
} from '../../frontend/src/app-shell/commands/workbench-binding.ts';
import { readAgentLoopSource, readAgentLoopTestsSource } from '../helpers/agent-loop-source.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const checklistPath = join(
  repoRoot,
  'tests/gates/ai-assistant-p4-smoke-checklist.md',
);
const pkg = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8'));
const testScript = pkg.scripts.test;

const REQUIRED_CHECKLIST_MARKERS = [
  '## UI 开窗',
  '## 配置保存',
  '## 主路径冒烟',
  '## Host / GLM 配置冒烟',
  '## J1 / execute 门闩内核验收',
  'H1 无 UI',
  '通用 Binding 夹具',
  'GLM',
  '关壳≠Reset',
  'Set 失败可恢复',
];

const LAYERED_VITEST = [
  'tests/gates/ai-assistant-window.test.js',
  'tests/app-shell/settings-llm.test.js',
  'tests/gates/ai-assistant-p4-smoke.test.js',
  'tests/gates/todo-desktop-surface-removed.test.js',
];

const HOST_TOOLS_MARKERS = [
  't3_agent_legacy_tools_rs_module_removed',
];

const LOOP_MARKERS = [
  'run_loop_clarify_under_limit_returns_none_not_wrote',
  'run_loop_host_empty_tools_rejects_tool_calls_without_dispatch',
  'run_loop_host_text_paths_remain_observable_without_tool_writes',
  'parallel_tool_calls_are_rejected_without_process_dispatch',
  'same_message_tool_calls_plus_content_does_not_dispatch_or_finalize',
  'replace_set_while_busy_clears_live_session',
  'unsupported_tool_calls_upstream_is_error_terminal_no_prompt_json',
  'length_and_http_errors_map_to_error_terminal_no_retry',
  'history_truncation_keeps_system_and_dual_hard_caps',
  'terminal_no_plan_unsupported_and_error_are_distinguishable',
  't3_host_business_chat_sends_empty_tools_to_llm',
  't3_host_empty_tools_facade_usable_confirmed',
  't2_empty_tools_binding_text_only_round_succeeds',
  't3_loop_rs_has_no_business_whitelist_dispatch_path',
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

describe('AI assistant P4 layered smoke gate', () => {
  it('ships executable smoke checklist with UI/config/main-path and Host/GLM steps', () => {
    expect(
      existsSync(checklistPath),
      'missing tests/gates/ai-assistant-p4-smoke-checklist.md',
    ).toBe(true);
    const text = readFileSync(checklistPath, 'utf8');
    for (const marker of REQUIRED_CHECKLIST_MARKERS) {
      expect(text, `checklist missing marker: ${marker}`).toContain(marker);
    }
  });

  it('npm test wires layered UI/config + this gate + todo desktop removal gate', () => {
    expect(testScript).toMatch(/vitest run --dir tests/);
    for (const file of LAYERED_VITEST) {
      expect(existsSync(join(repoRoot, file)), `missing ${file}`).toBe(true);
    }
    expect(existsSync(join(repoRoot, 'tests/todo-task'))).toBe(false);
  });

  it('Host Tools layer has update_master_title shell + validation coverage', () => {
    const toolsTests = readFileSync(
      join(repoRoot, 'src-tauri/src/unit-tests/agent/mod.rs'),
      'utf8',
    );
    for (const marker of HOST_TOOLS_MARKERS) {
      expect(toolsTests, `missing Host Tools test ${marker}`).toContain(
        `fn ${marker}`,
      );
    }
  });

  it('Loop layer covers clarify/write paths, busy rebind, failure taxonomy, history caps', () => {
    const loopTests = readAgentLoopTestsSource();
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

describe('Workbench Binding call surface (relocated from todo-task)', () => {
  let invokeMock;
  let events;

  beforeEach(() => {
    events = [];
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
          return { ok: false, code: 'set_invalid', state: 'unbound' };
        }
        return { ok: true, state: 'bound' };
      }
      if (cmd === 'execute_binding') {
        return { ok: false, code: 'rejected_unbound', state: 'unbound' };
      }
      if (cmd === 'present_ai_assistant') {
        return { surface: 'Present', window_label: 'ai-assistant' };
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

  it('Binding Set is key-only workbench; capability surface is Host MCP key', async () => {
    expect(WORKBENCH_BUSINESS_KEY).toBe('workbench');
    const result = await setWorkbenchBinding(trackCallbacks());
    expect(result.ok).toBe(true);
    expect(result.binding).toEqual({ key: WORKBENCH_BUSINESS_KEY });
    expect(result.binding).not.toHaveProperty('tools');
    expect(result.binding).not.toHaveProperty('prompt');
    expect(result.binding).not.toHaveProperty('callbacks');
    expect(invokeMock).toHaveBeenCalledWith('set_binding', {
      binding: { key: 'workbench' },
    });
  });

  it('Present without Set does not make execute succeed', async () => {
    const present = await window.__TAURI__.core.invoke('present_ai_assistant');
    expect(present.surface).toBe('Present');
    const exec = await window.__TAURI__.core.invoke('execute_binding');
    expect(exec.ok).toBe(false);
    expect(exec.code).toBe('rejected_unbound');
  });

  it('Set failure then retry recovers', async () => {
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'set_binding') {
        return { ok: false, code: 'set_invalid', state: 'unbound' };
      }
      return { ok: true, state: 'unbound' };
    });
    await expect(setWorkbenchBinding(trackCallbacks())).resolves.toMatchObject({
      ok: false,
      code: 'set_invalid',
    });
    expect(events.map((e) => e.event)).toEqual(['onError']);

    events.length = 0;
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'set_binding') {
        return { ok: true, state: 'bound' };
      }
      return {};
    });
    const recovered = await setWorkbenchBinding(trackCallbacks());
    expect(recovered.ok).toBe(true);
    expect(events.map((e) => e.event)).toEqual(['onBound']);
  });
});

describe('t6 layered acceptance L0/L1/L2 gate', () => {
  it('loop_tests lands L0/L1/L2 + leave-primary markers', () => {
    const loopTests = readAgentLoopTestsSource();
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
    const loopRs = readAgentLoopSource();
    expect(loopRs).toMatch(/LAYERED_ACCEPTANCE_L0/);
    expect(loopRs).toMatch(/LAYERED_ACCEPTANCE_L1/);
    expect(loopRs).toMatch(/LAYERED_ACCEPTANCE_L2/);
  });

  it('todo desktop module is gone; workbench binding relocated', () => {
    expect(existsSync(join(repoRoot, 'frontend/src/todo-task'))).toBe(false);
    expect(
      existsSync(join(repoRoot, 'frontend/src/app-shell/commands/workbench-binding.ts')),
    ).toBe(true);
  });
});
