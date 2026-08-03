// @vitest-environment jsdom
/**
 * t2 / P2 — Host Agent Loop business session tools empty (tech-doc T3/T6, AC2).
 * L1+L2: Binding call surface is key-only; Host Loop tools interface stays empty
 * (no in-process dispatch; MCP is the business capability plane).
 *
 * Layer map (T5):
 * - Interface layer: loop tools field / Binding.tools slot (empty for business path).
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

describe('t2 Host Agent empty tools — todos-binding key-only call surface', () => {
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

describe('t2 Host Agent empty tools — source / interface layer locks', () => {
  it('openai_tool_definitions_for_binding API retained (interface); dispatch removed (capability, t3)', () => {
    expect(toolsRs).toMatch(
      /pub fn openai_tool_definitions_for_binding\s*\(\s*tools:\s*&Value\s*\)/,
    );
    expect(toolsRs).toMatch(/pub fn openai_tool_definitions\s*\(/);
    expect(toolsRs).not.toMatch(/pub fn dispatch\s*\(/);
  });

  it('loop keeps tools field path and skips tool_calls-driven todos when tools empty', () => {
    // Host business path always passes empty tools to the LLM client (no Binding.tools defs).
    expect(loopRs).toMatch(
      /chat_completions\s*\(\s*&messages\s*,\s*&\s*\[\s*\]\s*,\s*config\s*\)/,
    );
    expect(loopRs).toMatch(/Unexpected tool_calls with empty request tools/);
    // Public key-only Set must clear Binding.tools (L1 empty-tools + L2 key-only).
    expect(loopRs).toMatch(/tools:\s*json!\(\[\]\)/);
    // No executable dispatch call sites (module docs/comments may still mention the name).
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
