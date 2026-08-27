import { existsSync, readdirSync, readFileSync } from 'node:fs';
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
const retiredJsPath = join(repoRoot, 'frontend/js/ai-assistant.js');
const homeHubPath = join(repoRoot, 'frontend/js/components/home-hub.js');
const capabilityPath = join(
  repoRoot,
  'src-tauri/capabilities/ai-assistant.json',
);
const defaultCapabilityPath = join(
  repoRoot,
  'src-tauri/capabilities/default.json',
);
const appCssPath = join(repoRoot, 'frontend/app.css');

describe('ai-assistant window shell (t5)', () => {
  it('retires independent-window HTML and overlay chat module', () => {
    expect(existsSync(htmlPath), 'frontend/ai-assistant.html retired').toBe(
      false,
    );
    expect(existsSync(retiredJsPath), 'frontend/js/ai-assistant.js retired').toBe(
      false,
    );
    expect(existsSync(homeHubPath), 'Home chat module').toBe(true);
    const js = readFileSync(homeHubPath, 'utf8');
    expect(js).toMatch(/mountHomeHub/);
    expect(js).toMatch(/list_chat_sessions|create_chat_session/);
  });

  it('Home chat layout styles live in the main application stylesheet', () => {
    const appCss = readFileSync(appCssPath, 'utf8');
    expect(appCss).toMatch(/\.home-chat-messages/);
    expect(appCss).toMatch(/\.home-chat-bubble--user/);
    expect(appCss).toMatch(/\.home-chat-composer/);
    expect(appCss).not.toMatch(/\.ai-assistant-panel/);
    expect(appCss).not.toMatch(/\.ai-assistant-fab\b/);
  });

  it('UI invokes agent_chat_turn and does not call plan write APIs', () => {
    const js = readFileSync(homeHubPath, 'utf8');
    expect(js).toMatch(/agent_chat_turn/);
    expect(js).not.toMatch(
      /add_todo_sub|update_todo_sub|update_todo_master_title|create_todo_task/,
    );
  });

  it('UI pulls get_ai_assistant_binding after list to hydrate the current session', () => {
    const js = readFileSync(homeHubPath, 'utf8');
    expect(js).toMatch(/get_ai_assistant_binding/);
    expect(aiAssistantCmd).toMatch(/get_ai_assistant_binding/);
    expect(libRs).toMatch(/get_ai_assistant_binding/);
  });

  it('SK-3 T5: binding exposes turns; Home hydrates without Reset', () => {
    const js = readFileSync(homeHubPath, 'utf8');
    const loopRs = readFileSync(
      join(repoRoot, 'src-tauri/src/services/agent/loop.rs'),
      'utf8',
    );
    const sessionRs = readFileSync(
      join(repoRoot, 'src-tauri/src/services/agent/session.rs'),
      'utf8',
    );
    expect(loopRs).toMatch(/get_ai_assistant_binding_core[\s\S]*turns/);
    expect(sessionRs).toMatch(/load_session/);
    expect(js).toMatch(/hydrateTurns/);
    expect(js).not.toMatch(/invoke\(\s*['"]reset_binding['"]/);
  });

  it('composer eligibility follows Binding Contract query_binding, not todo session', () => {
    const js = readFileSync(homeHubPath, 'utf8');
    expect(js).toMatch(/query_binding/);
    expect(js).toMatch(/ai-assistant:binding-changed/);
    expect(js).toMatch(/hostBound/);
    expect(js).toMatch(/Chat requires a workspace Binding/);
    expect(js).not.toMatch(/No todo bound/);
    expect(js).not.toMatch(/setComposerEnabled\(Boolean\(sessionId\)/);
  });

  it('Todos Set does not provision a chat session', () => {
    const todosBinding = readFileSync(
      join(repoRoot, 'frontend/js/todo-task/todos-binding.js'),
      'utf8',
    );
    expect(todosBinding).not.toMatch(/ensure_ai_assistant_session/);
    expect(todosBinding).not.toMatch(
      /invoke\(\s*['"]open_ai_assistant['"]/,
    );
    expect(aiAssistantCmd).toMatch(/ensure_ai_assistant_session/);
    expect(libRs).toMatch(/ensure_ai_assistant_session/);
  });

  it('retires create_or_focus and ai-assistant.html entry path from lib.rs', () => {
    // T4 / L16-T: independent WebviewUrl::App("ai-assistant.html") entry + create/focus helpers gone.
    expect(libRs).not.toMatch(/fn create_or_focus_ai_assistant_window\s*\(/);
    expect(libRs).not.toMatch(/fn ai_assistant_entry_path\s*\(/);
    expect(libRs).not.toMatch(/fn ai_assistant_spec\s*\(/);
    expect(libRs).not.toMatch(/struct AiAssistantWindowSpec/);
    expect(libRs).not.toMatch(/WebviewUrl::App\(\s*"ai-assistant\.html"/);
    expect(libRs).not.toMatch(/"ai-assistant\.html"/);
  });

  it('open_ai_assistant no longer wires create-or-focus window', () => {
    expect(aiAssistantCmd).not.toMatch(/create_or_focus_ai_assistant_window/);
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

  it('Home unmount does not invoke Binding Contract Reset', () => {
    const js = readFileSync(homeHubPath, 'utf8');
    expect(js).not.toMatch(/reset_binding/);
    expect(js).toMatch(/query_binding/);
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
      /fn set_binding_json[\s\S]*todo-task|fn execute_binding_json[\s\S]*角位/,
    );
  });

  it('T5 Home discards the current session on binding-changed Unbound', () => {
    const js = readFileSync(homeHubPath, 'utf8');
    expect(js).toMatch(/currentId\s*=\s*['"]['"]/);
    expect(js).toMatch(/ai-assistant:binding-changed/);
    expect(js).toMatch(/query_binding/);
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

  it('grants shell AI permissions on main; retires ai-assistant window capability island', () => {
    // T4 / L06-SC / L16-T: main hosts shell AI content; no orphan capability for retired window.
    expect(
      existsSync(capabilityPath),
      'capabilities/ai-assistant.json retired',
    ).toBe(false);
    expect(existsSync(defaultCapabilityPath), 'capabilities/default.json').toBe(
      true,
    );
    const capability = JSON.parse(readFileSync(defaultCapabilityPath, 'utf8'));
    expect(capability.identifier).toBe('default');
    expect(capability.windows).toContain('main');
    expect(capability.permissions).toContain('core:default');
    expect(capability.permissions).toContain('write-api');
    const capFiles = readdirSync(join(repoRoot, 'src-tauri/capabilities')).filter(
      (name) => name.endsWith('.json'),
    );
    for (const name of capFiles) {
      const cap = JSON.parse(
        readFileSync(join(repoRoot, 'src-tauri/capabilities', name), 'utf8'),
      );
      const windows = Array.isArray(cap.windows) ? cap.windows : [];
      if (windows.includes('ai-assistant') && !windows.includes('main')) {
        expect.fail(
          `${name} is a permission island for retired ai-assistant window`,
        );
      }
    }
  });
});

/**
 * T7 / L21-T / L22-VF — Present opens Home chat / no window / ensure / race / hydrate
 * (migrated from Present→overlay / create_or_focus window contract).
 */
describe('ai-assistant Present shell VF (t7)', () => {
  const mainJs = readFileSync(join(repoRoot, 'frontend/js/main.js'), 'utf8');
  const loopRs = readFileSync(
    join(repoRoot, 'src-tauri/src/services/agent/loop.rs'),
    'utf8',
  );

  it('Present opens Home only: main listens surface Present → navigate #/home', () => {
    expect(mainJs).toMatch(/ai-assistant:opened/);
    expect(mainJs).toMatch(/navigate\('#\/home'\)/);
    expect(mainJs).toMatch(/surface\s*===\s*['"]Present['"]|surface\s*===\s*"Present"/);
    expect(mainJs).not.toMatch(/handleAiAssistantOpenedPayload[\s\S]{0,400}presentNormalize/);
    // Migrated: must not call create_or_focus / WebviewWindow from Present path.
    expect(mainJs).not.toMatch(/create_or_focus_ai_assistant_window/);
    expect(mainJs).not.toMatch(/WebviewWindow/);
  });

  it('ensure does not open Home: only Present surface navigates', () => {
    // L09-AR / L22-VF ensure: session payloads sync only — no overlay / no Home jump.
    expect(mainJs).toMatch(/pendingPresentOpen|pullPendingPresentOpen/);
    expect(mainJs).toMatch(/ensure|session_id/);
    const handler = mainJs.match(
      /function\s+handleAiAssistantOpenedPayload[\s\S]*?^}/m,
    )?.[0] || mainJs.match(
      /ai-assistant:opened[\s\S]{0,1200}?surface[\s\S]{0,800}/,
    )?.[0];
    expect(handler, 'opened handler with surface branch').toBeTruthy();
    expect(handler).toMatch(/Present/);
    expect(handler).toMatch(/navigate\('#\/home'\)/);
    expect(handler).not.toMatch(/presentNormalize/);
  });

  it('Present race heal: pending_present Host flag + mount pull', () => {
    // L22-VF 竞态补开: Present before listen → pull pending after mount.
    expect(loopRs).toMatch(/pending_present/);
    expect(mainJs).toMatch(/pullPendingPresentOpen|pendingPresentOpen/);
    expect(mainJs).toMatch(/pending_present/);
    expect(aiAssistantCmd).toMatch(/get_ai_assistant_binding|pending_present/);
  });

  it('hydrate turns on binding read; Home unmount ≠ Reset', () => {
    const js = readFileSync(homeHubPath, 'utf8');
    expect(js).toMatch(/hydrateTurns/);
    expect(loopRs).toMatch(/get_ai_assistant_binding_core[\s\S]*turns/);
    expect(js).not.toMatch(/invoke\(\s*['"]reset_binding['"]/);
  });

  it('idempotent Present: Home navigation kept; no second window create path', () => {
    expect(mainJs).toMatch(/navigate\('#\/home'\)/);
    expect(aiAssistantCmd).not.toMatch(/create_or_focus_ai_assistant_window/);
    expect(libRs).not.toMatch(/fn create_or_focus_ai_assistant_window\s*\(/);
  });
});
