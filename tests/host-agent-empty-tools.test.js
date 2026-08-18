// @vitest-environment jsdom
/**
 * Host Agent Loop key-only Binding and MCP tool bridge.
 * Binding.tools remains empty, but the active MCP scene supplies model-callable
 * tools at turn time. In-process dispatch remains removed.
 *
 * Layer map (T5):
 * - Interface layer: Binding.tools remains empty; model tools come from MCP discovery.
 * - Capability layer: tools.rs dispatch removed (t3); business via MCP/HTTP only.
 * - Binding call surface: key-only Set (L2); Host registry loads MCP config.
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, vi } from 'vitest';

import {
  assembleTodosBindingBody,
  buildTodosBinding,
  TODOS_BUSINESS_KEY,
} from '../frontend/js/plan-task/todos-binding.js';

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

describe('Host Agent MCP tools — todos-binding key-only call surface', () => {
  it('assembleTodosBindingBody submits key-only (no tools/prompt/callbacks payload)', () => {
    const body = assembleTodosBindingBody();
    expect(body).toEqual({ key: TODOS_BUSINESS_KEY });
    expect(body).not.toHaveProperty('tools');
    expect(body).not.toHaveProperty('prompt');
    expect(body).not.toHaveProperty('callbacks');
  });

  it('buildTodosBinding Sets with key-only payload and observes onBound', async () => {
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

    const result = await buildTodosBinding(
      { masterTaskId: 'task_empty_tools' },
      {
        onBound: (p) => events.push({ event: 'onBound', payload: p }),
        onError: (p) => events.push({ event: 'onError', payload: p }),
      },
    );

    expect(result.ok).toBe(true);
    expect(result.binding).toEqual({ key: TODOS_BUSINESS_KEY });
    expect(invokeMock).toHaveBeenCalledWith(
      'set_binding',
      expect.objectContaining({
        binding: { key: TODOS_BUSINESS_KEY },
      }),
    );
    expect(events.map((e) => e.event)).toEqual(['onBound']);

    delete window.__TAURI__;
    vi.restoreAllMocks();
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

  it('derives model tools from active MCP while Binding.tools remains empty', () => {
    expect(loopRs).toMatch(/mcp_client::discover_tools\s*\(\s*&config\s*\)/);
    expect(loopRs).toMatch(/catalog\.definitions\.as_slice\(\)/);
    expect(loopRs).toMatch(
      /mcp_client::call_tool\s*\(\s*mcp_config\s*,\s*&call\.name\s*,\s*arguments\s*\)/,
    );
    expect(loopRs).not.toMatch(
      /chat_completions\s*\(\s*&messages\s*,\s*&\s*\[\s*\]\s*,\s*config\s*\)/,
    );
    // Public key-only Set still clears Binding.tools: tool definitions come
    // from the loaded MCP server, not caller-controlled Binding data.
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
