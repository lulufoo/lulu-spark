import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const libRs = readFileSync(join(repoRoot, 'src-tauri/src/lib.rs'), 'utf8');
const aiAssistantCmd = readFileSync(
  join(repoRoot, 'src-tauri/src/commands/ai_assistant.rs'),
  'utf8',
);
const htmlPath = join(repoRoot, 'frontend/ai-assistant.html');
const jsPath = join(repoRoot, 'frontend/js/ai-assistant.js');
const capabilityPath = join(
  repoRoot,
  'src-tauri/capabilities/ai-assistant.json',
);

describe('ai-assistant window shell (t5)', () => {
  it('ships ai-assistant.html and ai-assistant.js', () => {
    expect(existsSync(htmlPath), 'frontend/ai-assistant.html').toBe(true);
    expect(existsSync(jsPath), 'frontend/js/ai-assistant.js').toBe(true);
    const html = readFileSync(htmlPath, 'utf8');
    expect(html).toMatch(/ai-assistant\.js/);
    expect(html).toMatch(/ai-assistant-root/);
  });

  it('UI invokes agent_chat_turn and does not call plan write APIs', () => {
    const js = readFileSync(jsPath, 'utf8');
    expect(js).toMatch(/agent_chat_turn/);
    expect(js).not.toMatch(
      /add_todo_sub|update_todo_sub|update_todo_master_title|create_todo_task/,
    );
  });

  it('UI pulls get_ai_assistant_binding after listen to heal first-open race', () => {
    const js = readFileSync(jsPath, 'utf8');
    expect(js).toMatch(/get_ai_assistant_binding/);
    expect(aiAssistantCmd).toMatch(/get_ai_assistant_binding/);
    expect(libRs).toMatch(/get_ai_assistant_binding/);
  });

  it('composer eligibility follows Binding Contract query_binding, not todo session', () => {
    const js = readFileSync(jsPath, 'utf8');
    expect(js).toMatch(/query_binding/);
    expect(js).toMatch(/ai-assistant:binding-changed/);
    expect(js).toMatch(/hostBound/);
    expect(js).toMatch(/Unbound/);
    expect(js).not.toMatch(/No todo bound/);
    expect(js).not.toMatch(/setComposerEnabled\(Boolean\(sessionId\)/);
  });

  it('Todos Set provisions chat session without Present (ensure_ai_assistant_session)', () => {
    const todosBinding = readFileSync(
      join(repoRoot, 'frontend/js/plan-task/todos-binding.js'),
      'utf8',
    );
    expect(todosBinding).toMatch(/ensure_ai_assistant_session/);
    expect(todosBinding).not.toMatch(
      /invoke\(\s*['"]open_ai_assistant['"]/,
    );
    expect(aiAssistantCmd).toMatch(/ensure_ai_assistant_session/);
    expect(libRs).toMatch(/ensure_ai_assistant_session/);
  });

  it('defines create_or_focus_ai_assistant_window with focus/show', () => {
    expect(libRs).toMatch(/fn create_or_focus_ai_assistant_window\s*\(/);
    const fnBody = libRs.match(
      /fn create_or_focus_ai_assistant_window[\s\S]*?^}/m,
    )?.[0];
    expect(fnBody, 'create_or_focus_ai_assistant_window body').toBeTruthy();
    expect(fnBody).toMatch(/WebviewWindowBuilder::new/);
    expect(fnBody).toMatch(/ai-assistant/);
    expect(fnBody).toMatch(/ai-assistant\.html/);
    expect(fnBody).toMatch(/always_on_top\s*\(\s*true\s*\)/);
    expect(fnBody).toMatch(/\.show\s*\(/);
    expect(fnBody).toMatch(/set_focus\s*\(/);
    // Must not copy read-later "exists → skip" half-finished behavior alone.
    expect(fnBody).not.toMatch(/is_some\(\)\s*\{\s*return Ok\(\(\)\);\s*\}/);
  });

  it('open_ai_assistant wires create-or-focus', () => {
    expect(aiAssistantCmd).toMatch(/create_or_focus_ai_assistant_window/);
  });

  it('Present emits shell payload with entry_id and does not create_or_focus window', () => {
    // T3 / L09-AR / L16-T: Present = shell C_AI signal; tear down independent window create/focus.
    expect(aiAssistantCmd).toMatch(/present_ai_assistant/);
    expect(aiAssistantCmd).toMatch(/Present/);
    expect(aiAssistantCmd).toMatch(/entry_id/);
    expect(aiAssistantCmd).toMatch(/emit\(\s*EVENT_ASSISTANT_OPENED/);
    const presentFn = aiAssistantCmd.match(
      /pub async fn present_ai_assistant[\s\S]*?^}/m,
    )?.[0];
    expect(presentFn, 'present_ai_assistant body').toBeTruthy();
    expect(presentFn).not.toMatch(/create_or_focus_ai_assistant_window/);
    expect(presentFn).not.toMatch(/set_focus/);
    // Binding Contract ops remain Set/Reset/query/execute (+ callbacks) — not Present/Open.
    expect(aiAssistantCmd).toMatch(/set_binding_json/);
    expect(aiAssistantCmd).toMatch(/reset_binding_json/);
    expect(aiAssistantCmd).toMatch(/query_binding_json/);
    expect(aiAssistantCmd).toMatch(/execute_binding_json/);
    expect(libRs).toMatch(/present_ai_assistant/);
  });

  it('ensure opened payload stays session-only (no surface Present)', () => {
    // T3 / L09-AR: ensure vs Present share event name; surface discriminates openEntry.
    const ensureFn = aiAssistantCmd.match(
      /pub async fn ensure_ai_assistant_session[\s\S]*?^}/m,
    )?.[0];
    expect(ensureFn, 'ensure_ai_assistant_session body').toBeTruthy();
    expect(ensureFn).toMatch(/emit\(\s*EVENT_ASSISTANT_OPENED/);
    expect(ensureFn).not.toMatch(/create_or_focus_ai_assistant_window/);
    const loopRs = readFileSync(
      join(repoRoot, 'src-tauri/src/services/agent/loop.rs'),
      'utf8',
    );
    const ensureCore = loopRs.match(
      /pub fn ensure_chat_session_core[\s\S]*?^}/m,
    )?.[0];
    expect(ensureCore, 'ensure_chat_session_core body').toBeTruthy();
    expect(ensureCore).toMatch(/session_id/);
    expect(ensureCore).toMatch(/busy/);
    expect(ensureCore).not.toMatch(/"surface"\s*:\s*"Present"|surface:\s*"Present"/);
    expect(ensureCore).not.toMatch(/entry_id/);
  });

  it('shell dispose/close does not invoke Binding Contract Reset', () => {
    const js = readFileSync(jsPath, 'utf8');
    expect(js).not.toMatch(/reset_binding/);
    // Present≠bound: shell may observe Host Present, but must not treat open as Set.
    expect(js).toMatch(/Present/);
    expect(js).toMatch(
      /query_binding|binding_state|unbound|Present≠|Present !=|Present !==|not imply Set|does not imply Set/i,
    );
  });

  it('J1 / H1 acceptance is kernel-API driven, not business UI click as sole driver', () => {
    // Contract acceptance must live in Host unit fixtures, not plan-page entry clicks.
    const loopTests = readFileSync(
      join(repoRoot, 'src-tauri/src/unit-tests/services/agent/loop_tests.rs'),
      'utf8',
    );
    expect(loopTests).toMatch(/j1_generic_binding_fixture|j1_h1_kernel_api_fixture/);
    expect(loopTests).toMatch(/j1_1_legal_set_on_bound_execute_reset_rejects/);
    expect(loopTests).toMatch(/j1_2_illegal_set_keeps_state_no_on_bound_emits_set_invalid/);
    expect(loopTests).toMatch(/j1_5_contract_states_tools_prompt_callbacks_present_not_bound/);
    // Binding Contract ops surface remains Set/Reset/query/execute — not page click.
    expect(aiAssistantCmd).toMatch(/set_binding_json/);
    expect(aiAssistantCmd).toMatch(/execute_binding_json/);
    expect(aiAssistantCmd).not.toMatch(
      /fn set_binding_json[\s\S]*plan-task|fn execute_binding_json[\s\S]*角位/,
    );
  });

  it('T5 shell discards cached sessionId on binding-changed Unbound/new Bound', () => {
    const js = readFileSync(jsPath, 'utf8');
    // Unbound / new Bound must clear cached sessionId; composer stays gated by query_binding/hostBound.
    expect(js).toMatch(/sessionId\s*=\s*['"]['"]/);
    expect(js).toMatch(/applyHostContractState/);
    expect(js).toMatch(/ai-assistant:binding-changed/);
    expect(js).toMatch(/query_binding/);
    // Must not keep driving execute from a stale cached session after cut/rebind.
    expect(js).toMatch(/hostBound/);
  });

  it('T5 defensive cut command path emits binding-changed like Set/Reset', () => {
    expect(aiAssistantCmd).toMatch(/defensive_unbound/);
    expect(aiAssistantCmd).toMatch(/EVENT_BINDING_CHANGED/);
    // Defensive path must not be core-only: command wrapper emits the shell event.
    expect(aiAssistantCmd).toMatch(
      /defensive_unbound[\s\S]{0,400}?emit\(\s*EVENT_BINDING_CHANGED/,
    );
    expect(libRs).toMatch(/defensive_unbound/);
  });

  it('registers ai-assistant capability with write-api for agent_chat_turn', () => {
    expect(existsSync(capabilityPath), 'capabilities/ai-assistant.json').toBe(
      true,
    );
    const capability = JSON.parse(readFileSync(capabilityPath, 'utf8'));
    expect(capability.identifier).toBe('ai-assistant');
    expect(capability.windows).toContain('ai-assistant');
    expect(capability.permissions).toContain('core:default');
    expect(capability.permissions).toContain('write-api');
  });
});
