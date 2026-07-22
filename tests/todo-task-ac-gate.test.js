/**
 * T10 end-to-end acceptance gate (tech-doc SK-5 / AC-Host·MCP / AC-SKILL /
 * AC-等价 / AC-迁移 / AC-可观测 / AC-测试残留).
 *
 * Hard cut: no plan_* tools/aliases. Any falsifying signal fails this suite.
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

/** tech-doc AC-等价 — full 12-tool set (complete/link/attachment required). */
const EQUIVALENCE_TODO_TOOLS = [
  'create_todo_task',
  'list_todo_tasks',
  'get_todo_task',
  'delete_todo_task',
  'add_todo_sub',
  'delete_todo_sub',
  'complete_todo',
  'link_todo_archive',
  'add_todo_attachment',
  'list_todo_attachments',
  'get_todo_attachment',
  'update_todo_attachment',
];

const FORBIDDEN_PLAN_TOOLS = [
  'create_plan_task',
  'list_plan_tasks',
  'get_plan_task',
  'delete_plan_task',
  'add_plan_sub',
  'delete_plan_sub',
  'complete_plan',
  'complete_plan_sub',
  'link_plan_archive',
  'add_plan_attachment',
  'list_plan_attachments',
  'get_plan_attachment',
  'update_plan_attachment',
];

function read(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

function extractQuotedToolList(src, constName) {
  const re = new RegExp(`const ${constName}\\s*=\\s*\\[([\\s\\S]*?)\\];`);
  const m = src.match(re);
  if (!m) return null;
  return [...m[1].matchAll(/['"]([^'"]+)['"]/g)].map((x) => x[1]);
}

describe('T10 — AC-Host/MCP (hard cut, no plan_* )', () => {
  it('MCP index registers only todo_* task tools; complete_plan_sub absent', () => {
    const src = read('packages/knowledge-mcp/index.mjs');
    for (const tool of EQUIVALENCE_TODO_TOOLS) {
      expect(src, `missing ${tool}`).toMatch(new RegExp(`['"]${tool}['"]`));
    }
    for (const tool of FORBIDDEN_PLAN_TOOLS) {
      expect(src, `forbidden ${tool}`).not.toMatch(
        new RegExp(`registerTool\\(\\s*['"]${tool}['"]`),
      );
    }
    expect(src).not.toMatch(/registerTool\(\s*['"]complete_plan_sub['"]/);
  });

  it('verify.mjs forbids plan_* tool registration and lists the 12-tool equivalence set', () => {
    const src = read('packages/knowledge-mcp/scripts/verify.mjs');
    expect(src).toMatch(/EQUIVALENCE_TODO_TOOLS|T10_EQUIVALENCE_TODO_TOOLS/);
    const names =
      extractQuotedToolList(src, 'EQUIVALENCE_TODO_TOOLS') ||
      extractQuotedToolList(src, 'T10_EQUIVALENCE_TODO_TOOLS');
    expect(names, 'verify.mjs must declare EQUIVALENCE_TODO_TOOLS').toBeTruthy();
    expect([...names].sort()).toEqual([...EQUIVALENCE_TODO_TOOLS].sort());
    for (const tool of FORBIDDEN_PLAN_TOOLS) {
      expect(src, `verify must forbid ${tool}`).toContain(`'${tool}'`);
    }
    expect(src).toContain('forbidden plan_* tool still registered');
    expect(src).toContain('complete_plan_sub');
  });
});

describe('T10 — AC-等价 (full 12-tool set in e2e + verify)', () => {
  it('todo-task-mcp-e2e lists and exercises all 12 equivalence tools incl. complete/link/attachment', () => {
    const src = read('packages/knowledge-mcp/scripts/todo-task-mcp-e2e.mjs');
    expect(src).toMatch(/EQUIVALENCE_TODO_TOOLS|T10_EQUIVALENCE_TODO_TOOLS/);
    const names =
      extractQuotedToolList(src, 'EQUIVALENCE_TODO_TOOLS') ||
      extractQuotedToolList(src, 'T10_EQUIVALENCE_TODO_TOOLS') ||
      extractQuotedToolList(src, 'TODO_TOOLS');
    expect(names, 'e2e must declare the 12-tool equivalence set').toBeTruthy();
    expect([...names].sort()).toEqual([...EQUIVALENCE_TODO_TOOLS].sort());

    for (const tool of [
      'complete_todo',
      'link_todo_archive',
      'add_todo_attachment',
      'list_todo_attachments',
      'get_todo_attachment',
      'update_todo_attachment',
    ]) {
      expect(src, `e2e must callTool ${tool}`).toMatch(
        new RegExp(`callTool\\(\\{\\s*name:\\s*['"]${tool}['"]`),
      );
    }
  });

  it('verify.mjs exercises complete_todo, link_todo_archive, and attachment quartet', () => {
    const src = read('packages/knowledge-mcp/scripts/verify.mjs');
    for (const tool of [
      'complete_todo',
      'link_todo_archive',
      'add_todo_attachment',
      'list_todo_attachments',
      'get_todo_attachment',
      'update_todo_attachment',
    ]) {
      expect(src, `verify must callTool ${tool}`).toMatch(
        new RegExp(`callTool\\(\\{\\s*name:\\s*['"]${tool}['"]`),
      );
    }
  });
});

describe('T10 — AC-SKILL (todo-task drives new tools; no /plan-task)', () => {
  it('sibling skills worktree exposes todo-task SKILL with todo_* tools only', () => {
    const skillsRoot = join(repoRoot, '..', '475641f1-4be0-lulu-workbench-skills');
    const skillMd = join(skillsRoot, 'todo-task', 'SKILL.md');
    expect(existsSync(skillMd), `missing ${skillMd}`).toBe(true);
    expect(existsSync(join(skillsRoot, 'plan-task')), 'plan-task dir must not remain').toBe(
      false,
    );
    const src = readFileSync(skillMd, 'utf8');
    expect(src).toMatch(/^name:\s*todo-task\s*$/m);
    expect(src).not.toMatch(/\/plan-task/);
    for (const tool of EQUIVALENCE_TODO_TOOLS) {
      expect(src, `SKILL missing ${tool}`).toContain(tool);
    }
    expect(src).toContain('complete_plan_sub');
    expect(src).toMatch(/do not call|unavailable|removed/i);
  });
});

describe('T10 — AC-迁移 (gate marker; no plan.md/plan_tasks residue on success)', () => {
  it('migrate vitest locks success-path residue + .migration_gate_passed', () => {
    const src = read('tests/migrate-plan-tasks-to-todo-tasks.test.js');
    expect(src).toContain('.migration_gate_passed');
    expect(src).toMatch(/existsSync\(join\(wbRoot,\s*'plan_tasks'\)\)\)\.toBe\(false\)/);
    expect(src).toMatch(
      /existsSync\(join\(wbRoot,\s*'todo_tasks',\s*'tasks',\s*taskId,\s*'plan\.md'\)\)\)\.toBe\(false\)/,
    );
    expect(src).toMatch(
      /existsSync\(join\(wbRoot,\s*'todo_tasks',\s*GATE\)\)\)\.toBe\(true\)/,
    );
  });

  it('Host cold-start gate reads durable .migration_gate_passed (not in-process exit)', () => {
    const service = read('src-tauri/src/services/todo_task/mod.rs');
    expect(service).toContain('MIGRATION_GATE_FILE');
    expect(service).toContain('.migration_gate_passed');
    expect(service).toContain('migration_gate_passed');
    const http = read('src-tauri/src/services/local_http/mod.rs');
    expect(http).toMatch(/migration_gate|MigGate|gated/i);
    const httpTests = read('src-tauri/src/unit-tests/services/local_http.rs');
    expect(httpTests).toContain('.migration_gate_passed');
    expect(httpTests).toMatch(/missing .*gate|gate_passed_marker_opens/i);
  });
});

describe('T10 — AC-可观测 (migrate success/failure distinguishable)', () => {
  it('migrate vitest requires structured failure logs with task id + reason', () => {
    const src = read('tests/migrate-plan-tasks-to-todo-tasks.test.js');
    expect(src).toContain('failure logs are structured with task id and reason');
    expect(src).toContain('findJsonLog');
    expect(src).toMatch(/task_id|taskId/);
    expect(src).toMatch(/reason|\.error|\.message/);
    expect(src).toContain('exit non-zero must not coexist with a valid .migration_gate_passed');
  });

  it('migrate script emits structured JSON success/failure lines', () => {
    const script = read('scripts/migrate-plan-tasks-to-todo-tasks');
    expect(script).toMatch(/"event"\s*:|"status"\s*:|"task_id"\s*:/);
    expect(script).toMatch(/success|ok|failed|error/i);
  });
});

describe('T10 — AC-测试残留 (old contract names purged from delivery tests)', () => {
  it('delivery e2e entry is todo-task-mcp-e2e; plan-task-mcp-e2e removed', () => {
    expect(existsSync(join(repoRoot, 'packages/knowledge-mcp/scripts/todo-task-mcp-e2e.mjs'))).toBe(
      true,
    );
    expect(existsSync(join(repoRoot, 'packages/knowledge-mcp/scripts/plan-task-mcp-e2e.mjs'))).toBe(
      false,
    );
  });

  it('e2e and verify do not register or alias plan_* tool names', () => {
    const e2e = read('packages/knowledge-mcp/scripts/todo-task-mcp-e2e.mjs');
    const verify = read('packages/knowledge-mcp/scripts/verify.mjs');
    for (const src of [e2e, verify]) {
      expect(src).toMatch(/FORBIDDEN_PLAN_/);
      expect(src).not.toMatch(/registerTool\(\s*['"]create_plan_task['"]/);
      expect(src).not.toMatch(/name:\s*['"]create_plan_task['"]/);
      expect(src).not.toMatch(/name:\s*['"]complete_plan_sub['"]/);
    }
  });

  it('npm test includes this T10 ac-gate file', () => {
    const pkg = JSON.parse(read('package.json'));
    expect(pkg.scripts.test).toContain('tests/todo-task-ac-gate.test.js');
  });
});
