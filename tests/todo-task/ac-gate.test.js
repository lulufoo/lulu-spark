/**
 * T10 end-to-end acceptance gate (tech-doc SK-5 / AC-Host·MCP / AC-SKILL /
 * AC-等价 / AC-迁移 / AC-可观测 / AC-测试残留).
 *
 * Hard cut: no plan_* tools/aliases. Host MCP is runtime SSOT (T10 archive).
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { readRsPath } from '../helpers/read-rs-dir.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const HOST_ADAPTER = 'src-tauri/src/mcp_host';

/** tech-doc AC-等价 — full 13-tool set (update/complete/link/attachment required). */
const EQUIVALENCE_TODO_TOOLS = [
  'create_todo_task',
  'update_todo_task',
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
  const abs = join(repoRoot, rel);
  return rel.includes('src-tauri/') ? readRsPath(abs) : readFileSync(abs, 'utf8');
}

function extractQuotedToolList(src, constName) {
  const re = new RegExp(`const ${constName}\\s*=\\s*\\[([\\s\\S]*?)\\];`);
  const m = src.match(re);
  if (!m) return null;
  return [...m[1].matchAll(/['"]([^'"]+)['"]/g)].map((x) => x[1]);
}

function hostToolNames(src) {
  return [...src.matchAll(/route\(\s*"([^"]+)"/g)].map((m) => m[1]);
}

describe('T10 — AC-Host/MCP (hard cut, no plan_* )', () => {
  it('Host adapter registers only todo_* task tools; complete_plan_sub absent', () => {
    const src = read(HOST_ADAPTER);
    const names = hostToolNames(src);
    for (const tool of EQUIVALENCE_TODO_TOOLS) {
      expect(names, `missing ${tool}`).toContain(tool);
    }
    for (const tool of FORBIDDEN_PLAN_TOOLS) {
      expect(names, `forbidden ${tool}`).not.toContain(tool);
    }
    expect(names).not.toContain('complete_plan_sub');
  });

  it('verify-host-mcp targets Host URL and does not spawn Node index.mjs', () => {
    const src = read('scripts/verify-host-mcp.mjs');
    expect(src).toContain('http://127.0.0.1:9876');
    expect(src).toMatch(/todo_task/);
    expect(src).toMatch(/cursor_ide/);
    expect(src).not.toMatch(/['"]packages\/knowledge-mcp\/index\.mjs['"]/);
    expect(src).not.toMatch(/spawnSidecar\s*\(/);
  });
});

describe('T10 — AC-等价 (full 13-tool set in e2e)', () => {
  it('todo-task-mcp-e2e lists and exercises all 13 equivalence tools incl. complete/link/attachment', () => {
    const src = read('scripts/todo-task-mcp-e2e.mjs');
    expect(src).toMatch(/EQUIVALENCE_TODO_TOOLS|T10_EQUIVALENCE_TODO_TOOLS/);
    const names =
      extractQuotedToolList(src, 'EQUIVALENCE_TODO_TOOLS') ||
      extractQuotedToolList(src, 'T10_EQUIVALENCE_TODO_TOOLS') ||
      extractQuotedToolList(src, 'TODO_TOOLS');
    expect(names, 'e2e must declare the 13-tool equivalence set').toBeTruthy();
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

  it('Host adapter routes equivalence tools to Sidecar /api/* paths', () => {
    const src = read(HOST_ADAPTER);
    for (const tool of [
      'complete_todo',
      'link_todo_archive',
      'add_todo_attachment',
      'list_todo_attachments',
      'get_todo_attachment',
      'update_todo_attachment',
    ]) {
      expect(src, `Host missing route for ${tool}`).toMatch(
        new RegExp(`route\\(\\s*"${tool}"`),
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
    const src = read('tests/gates/migrate-plan-tasks.test.js');
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
    const service = read('src-tauri/src/services/todo_task/migrate.rs');
    expect(service).toContain('MIGRATION_GATE_FILE');
    expect(service).toContain('.migration_gate_passed');
    expect(service).toContain('migration_gate_passed');
    const http = read('src-tauri/src/main_host');
    expect(http).toMatch(/migration_gate|MigGate|gated/i);
    const httpTests = read('src-tauri/src/unit-tests/main_host.rs');
    expect(httpTests).toContain('.migration_gate_passed');
    expect(httpTests).toMatch(/missing .*gate|gate_passed_marker_opens/i);
  });
});

describe('T10 — AC-可观测 (migrate success/failure distinguishable)', () => {
  it('migrate vitest requires structured failure logs with task id + reason', () => {
    const src = read('tests/gates/migrate-plan-tasks.test.js');
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
  it('delivery e2e entry is scripts/todo-task-mcp-e2e; plan-task-mcp-e2e removed', () => {
    expect(existsSync(join(repoRoot, 'scripts/todo-task-mcp-e2e.mjs'))).toBe(true);
    expect(existsSync(join(repoRoot, 'scripts/plan-task-mcp-e2e.mjs'))).toBe(false);
    expect(existsSync(join(repoRoot, 'packages/knowledge-mcp/index.mjs'))).toBe(false);
  });

  it('e2e forbids plan_* tool names; Host adapter does not register them', () => {
    const e2e = read('scripts/todo-task-mcp-e2e.mjs');
    expect(e2e).toMatch(/FORBIDDEN_PLAN_/);
    expect(e2e).not.toMatch(/name:\s*['"]create_plan_task['"]/);
    expect(e2e).not.toMatch(/name:\s*['"]complete_plan_sub['"]/);
    const host = read(HOST_ADAPTER);
    for (const tool of FORBIDDEN_PLAN_TOOLS) {
      expect(host).not.toMatch(new RegExp(`route\\(\\s*"${tool}"`));
    }
  });

  it('npm test includes this T10 ac-gate file', () => {
    const pkg = JSON.parse(read('package.json'));
    expect(pkg.scripts.test).toMatch(/vitest run --dir tests/);
  });
});
