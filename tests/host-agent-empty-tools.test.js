// @vitest-environment jsdom
/**
 * Host Agent Loop key-only Binding and MCP tool bridge.
 * Binding.tools remains empty. Model tools come from MCP discovery plus Host
 * file tools (grep/read/write/edit) gated by the Set-time path fence.
 * In-process business dispatch remains removed.
 *
 * Layer map (t2):
 * - Interface layer: Binding.tools remains empty; model tools come from MCP discovery.
 * - Capability layer: tools.rs dispatch removed (t3); business via MCP/HTTP only.
 * - Binding call surface: key-only workbench Set (C7_4b-T).
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, vi } from 'vitest';

import {
  setWorkbenchBinding,
  WORKBENCH_BUSINESS_KEY,
} from '../frontend/js/todo-task/todos-binding.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const toolsRs = readFileSync(
  join(fixtureRoot, 'src-tauri/src/services/agent/tools.rs'),
  'utf8',
);
const loopRs = readFileSync(
  join(fixtureRoot, 'src-tauri/src/services/agent/loop.rs'),
  'utf8',
);
const packageJson = JSON.parse(
  readFileSync(join(fixtureRoot, 'package.json'), 'utf8'),
);
const todosBindingJs = readFileSync(
  join(fixtureRoot, 'frontend/js/todo-task/todos-binding.js'),
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
  it('openai_tool_definitions_for_binding API retained (interface); dispatch removed (capability, t3)', () => {
    expect(toolsRs).toMatch(
      /pub fn openai_tool_definitions_for_binding\s*\(\s*tools:\s*&Value\s*\)/,
    );
    expect(toolsRs).toMatch(/pub fn openai_tool_definitions\s*\(/);
    expect(toolsRs).not.toMatch(/pub fn dispatch\s*\(/);
  });

  it('derives model tools from active MCP plus Host file tools while Binding.tools remains empty', () => {
    expect(loopRs).toMatch(/mcp_client::discover_tools\s*\(\s*&config\s*\)/);
    expect(loopRs).toMatch(/catalog\.definitions\.as_slice\(\)/);
    expect(loopRs).toMatch(
      /mcp_client::call_tool\s*\(\s*mcp_config\s*,\s*&call\.name\s*,\s*arguments\s*\)/,
    );
    expect(loopRs).toMatch(/fs_tools::call\s*\(/);
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
    expect(packageJson.scripts.test).toMatch(/host-agent-empty-tools\.test\.js/);
  });
});
