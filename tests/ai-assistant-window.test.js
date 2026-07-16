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
      /add_plan_sub|update_plan_sub|update_plan_master_title|create_plan_task/,
    );
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
