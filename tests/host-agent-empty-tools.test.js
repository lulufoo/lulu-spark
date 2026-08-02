// @vitest-environment jsdom
/**
 * t2 / P2 — Host Agent Loop business session tools empty (tech-doc T3/T6, AC2).
 *
 * Layer map (T5):
 * - Interface layer: loop tools field / Binding.tools slot (may be empty array).
 * - Capability layer: tools.rs dispatch (still present; deleted in t3 — not this task).
 * - MCP/HTTP: knowledge-mcp + local_http (t1).
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, vi } from 'vitest';

import {
  assembleTodosBindingBody,
  buildTodosBinding,
  TODOS_T_LIFT_TOOL_NAMES,
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

describe('t2 Host Agent empty tools — todos-binding assemble contract', () => {
  it('assembleTodosBindingBody submits tools: [] (no T-lift business handles)', () => {
    const body = assembleTodosBindingBody('task_empty_tools');
    expect(Array.isArray(body.tools)).toBe(true);
    expect(body.tools).toEqual([]);
    expect(body.prompt).toBeTruthy();
    expect(body.callbacks).toEqual({});
    for (const name of TODOS_T_LIFT_TOOL_NAMES) {
      expect(JSON.stringify(body.tools)).not.toContain(name);
    }
  });

  it('buildTodosBinding Sets with empty tools array and observes onBound', async () => {
    const events = [];
    const invokeMock = vi.fn(async (cmd, args) => {
      if (cmd === 'set_binding') {
        const binding = args?.binding;
        if (
          !binding ||
          !Array.isArray(binding.tools) ||
          binding.prompt == null ||
          binding.callbacks == null
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
    expect(result.binding.tools).toEqual([]);
    expect(invokeMock).toHaveBeenCalledWith(
      'set_binding',
      expect.objectContaining({
        binding: expect.objectContaining({ tools: [] }),
      }),
    );
    expect(events.map((e) => e.event)).toEqual(['onBound']);

    delete window.__TAURI__;
    vi.restoreAllMocks();
  });
});

describe('t2 Host Agent empty tools — source / interface layer locks', () => {
  it('openai_tool_definitions_for_binding API retained (interface); dispatch still present (capability, t3)', () => {
    expect(toolsRs).toMatch(
      /pub fn openai_tool_definitions_for_binding\s*\(\s*tools:\s*&Value\s*\)/,
    );
    expect(toolsRs).toMatch(/pub fn openai_tool_definitions\s*\(/);
    expect(toolsRs).toMatch(/pub fn dispatch\s*\(/);
  });

  it('loop keeps tools field path and skips tool_calls-driven todos when tools empty', () => {
    expect(loopRs).toMatch(
      /openai_tool_definitions_for_binding\s*\(\s*&binding\.tools\s*\)/,
    );
    expect(loopRs).toMatch(
      /tools_defs\.is_empty\(\)|!tools_defs\.is_empty\(\)/,
    );
  });

  it('npm test includes this empty-tools suite', () => {
    expect(packageJson.scripts.test).toMatch(/host-agent-empty-tools\.test\.js/);
  });
});
