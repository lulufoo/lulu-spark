/**
 * T7 / P4 layered coverage gate: Host Tools + Loop automated markers,
 * UI/config/main-path + dual-platform smoke checklist, plan-task-write regression wiring.
 * SK-4: J1 / execute 门闩内核验收夹具 markers (H1 无业务 UI 驱动).
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

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
  'H1 无 UI',
  '通用 Binding 夹具',
  'KIMI',
  'GLM',
];

const LAYERED_VITEST = [
  'tests/ai-assistant-window.test.js',
  'tests/llm-settings.test.js',
  'tests/plan-task-ai-assistant-entry.test.js',
  'tests/plan-task-write.test.js',
  'tests/ai-assistant-p4-smoke.test.js',
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
