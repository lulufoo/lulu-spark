// @vitest-environment jsdom
/**
 * Host Agent Loop key-only Binding and MCP tool bridge.
 * Binding.tools remains empty. Model tools come from MCP discovery plus Host
 * file tools (grep/read/write/str_replace) gated by the Set-time path fence.
 * Legacy in-process OpenAI plan tool defs (`agent/tools.rs`) are removed.
 *
 * Layer map:
 * - Interface layer: Binding.tools remains empty; model tools come from MCP discovery.
 * - Capability layer: business via MCP/HTTP only; no agent/tools.rs.
 * - Binding call surface: key-only workbench Set (C7_4b-T).
 */
import { readFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, vi } from 'vitest';

import {
  setWorkbenchBinding,
  WORKBENCH_BUSINESS_KEY,
} from '../../frontend/src/todo-task/commands/binding.ts';
import { readAgentLoopSource } from '../helpers/agent-loop-source.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const toolsRsPath = join(fixtureRoot, 'src-tauri/src/agent/tools.rs');
const agentModRs = readFileSync(
  join(fixtureRoot, 'src-tauri/src/agent/mod.rs'),
  'utf8',
);
const loopRs = readAgentLoopSource();
const packageJson = JSON.parse(
  readFileSync(join(fixtureRoot, 'package.json'), 'utf8'),
);
const todosBindingJs = readFileSync(
  join(fixtureRoot, 'frontend/src/todo-task/commands/binding.ts'),
  'utf8',
);

describe('Host Agent MCP tools — workbench key-only call surface', () => {
  it('setWorkbenchBinding submits key-only workbench (no tools/prompt/callbacks payload)', async () => {
    expect(WORKBENCH_BUSINESS_KEY).toBe('workbench');
    const events = [];
    const invokeMock = vi.fn(async (cmd, args) => {
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
      if (cmd === 'ensure_ai_assistant_session') {
        return { session_id: 'sess_empty', busy: false };
      }
      return {};
    });
    window.__TAURI__ = { core: { invoke: invokeMock } };

    const result = await setWorkbenchBinding({
      onBound: (p) => events.push({ event: 'onBound', payload: p }),
      onError: (p) => events.push({ event: 'onError', payload: p }),
    });

    expect(result.ok).toBe(true);
    expect(result.binding).toEqual({ key: WORKBENCH_BUSINESS_KEY });
    expect(result.binding).not.toHaveProperty('tools');
    expect(result.binding).not.toHaveProperty('prompt');
    expect(result.binding).not.toHaveProperty('callbacks');
    expect(invokeMock).toHaveBeenCalledWith('set_binding', {
      binding: { key: WORKBENCH_BUSINESS_KEY },
    });
    expect(events.map((e) => e.event)).toEqual(['onBound']);

    delete window.__TAURI__;
    vi.restoreAllMocks();
  });

  it('helper module no longer exports todos/notes Binding assemblers', () => {
    expect(todosBindingJs).not.toMatch(/\bassembleTodosBindingBody\b/);
    expect(todosBindingJs).not.toMatch(/\bbuildTodosBinding\b/);
    expect(todosBindingJs).not.toMatch(/\bTODOS_BUSINESS_KEY\b/);
  });
});

describe('Host Agent MCP tools — source / interface layer locks', () => {
  it('legacy agent/tools.rs OpenAI plan defs are removed; tools/ module owns catalog', () => {
    expect(existsSync(toolsRsPath)).toBe(false);
    expect(agentModRs).toMatch(/pub mod tools/);
    expect(agentModRs).toMatch(/pub mod mcp/);
    expect(agentModRs).not.toMatch(/pub mod fs_tools/);
    expect(agentModRs).not.toMatch(/pub mod mcp_client/);
    expect(agentModRs).not.toMatch(/openai_tool_definitions/);
    expect(agentModRs).toMatch(/WORKBENCH_HOST_SYSTEM_PROMPT/);
  });

  it('derives model tools from active MCP plus Host file tools while Binding.tools remains empty', () => {
    expect(loopRs).toMatch(/tools::discover_and_merge\s*\(/);
    expect(loopRs).toMatch(/tools::invoke\s*\(/);
    expect(loopRs).toMatch(/catalog\.definitions\.as_slice\(\)/);
    expect(loopRs).not.toMatch(/mcp_client::/);
    expect(loopRs).not.toMatch(/fs_tools::/);
    expect(loopRs).toMatch(/workbench_path_fence::expand_for_business_key/);
    expect(loopRs).not.toMatch(
      /chat_completions\s*\(\s*&messages\s*,\s*&\s*\[\s*\]\s*,\s*config\s*\)/,
    );
    // Public key-only Set still clears Binding.tools: tool definitions come
    // from the loaded MCP server plus Host file tools, not caller-controlled Binding data.
    expect(loopRs).toMatch(/tools:\s*json!\(\[\]\)/);
    // No executable process-local dispatch call sites.
    const dispatchCallLines = loopRs
      .split('\n')
      .filter((line) => {
        const trimmed = line.trim();
        if (trimmed.startsWith('//') || trimmed.startsWith('//!')) return false;
        return /tools::dispatch\s*\(/.test(line);
      });
    expect(dispatchCallLines).toEqual([]);
  });

  it('npm test includes this empty-tools suite', () => {
    expect(packageJson.scripts.test).toMatch(/vitest run --dir tests/);
  });
});
