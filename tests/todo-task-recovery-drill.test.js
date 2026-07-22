/**
 * T11 recovery drill gate (tech-doc SK-5 / T11 / AC-恢复).
 *
 * Documents + falsifies automatic reverse migration. Ops checklist must exist;
 * Host must not auto-invoke migrate at App start; missing gate gates todo API only.
 */
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const DRILL_DOC = 'docs/features/plan-task-todos-contract/recovery-drill-checklist.md';

function read(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

function listFilesRecursive(dir, out = []) {
  if (!existsSync(dir)) return out;
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) {
      if (name === 'node_modules' || name === 'target' || name === '.git') continue;
      listFilesRecursive(full, out);
    } else {
      out.push(full);
    }
  }
  return out;
}

describe('T11 — AC-恢复 ops drill checklist', () => {
  it('recovery drill checklist exists under plan-task-todos-contract', () => {
    expect(existsSync(join(repoRoot, DRILL_DOC)), `missing ${DRILL_DOC}`).toBe(true);
  });

  it('checklist records code rollback, SKILL reinstall + Reload MCP, ops backup restore', () => {
    const doc = read(DRILL_DOC);
    expect(doc).toMatch(/代码回滚|回滚 Host|回滚.*MCP|git checkout|git revert/i);
    expect(doc).toMatch(/回装.*SKILL|旧 SKILL|todo-task|plan-task/);
    expect(doc).toMatch(/Reload MCP|重载 MCP/i);
    expect(doc).toMatch(/运维备份|备份恢复|ops backup/i);
    expect(doc).toMatch(/无自动反向|不提供自动反向|no automatic reverse/i);
  });

  it('checklist records single migrate failure does not block App start (API gate only)', () => {
    const doc = read(DRILL_DOC);
    expect(doc).toMatch(/不阻断.*App|不阻断.*启动|App 启动/);
    expect(doc).toMatch(/门闩|migration_gate|\.migration_gate_passed|todo API/);
  });
});

describe('T11 — no automatic reverse migration (falsifier)', () => {
  it('repo has no todo_tasks→plan_tasks / todo.md→plan.md reverse migrate script', () => {
    const scriptsDir = join(repoRoot, 'scripts');
    const names = existsSync(scriptsDir) ? readdirSync(scriptsDir) : [];
    const forbidden = names.filter((n) =>
      /migrate.*todo.*plan|reverse.*migrat|migrat.*reverse|todo-tasks-to-plan/i.test(n),
    );
    expect(forbidden, `forbidden reverse scripts: ${forbidden.join(', ')}`).toEqual([]);

    const migrateSrc = read('scripts/migrate-plan-tasks-to-todo-tasks');
    expect(migrateSrc).toMatch(/Not invoked at application startup/);
    // Forward-only path constants; must not rewrite NEW→OLD.
    expect(migrateSrc).toContain('OLD_ROOT = "plan_tasks"');
    expect(migrateSrc).toContain('NEW_ROOT = "todo_tasks"');
    expect(migrateSrc).not.toMatch(/NEW_ROOT.*OLD_ROOT|todo_tasks.*→.*plan_tasks/);
    expect(migrateSrc).not.toMatch(/rename\(\s*new\s*,\s*old\s*\)|new\.rename\(old\)/);
  });

  it('Host/runtime never spawns migrate-plan-tasks-to-todo-tasks (no auto migrate / reverse)', () => {
    const rustFiles = listFilesRecursive(join(repoRoot, 'src-tauri', 'src')).filter((p) =>
      p.endsWith('.rs'),
    );
    const offenders = [];
    for (const file of rustFiles) {
      const src = readFileSync(file, 'utf8');
      if (/migrate-plan-tasks-to-todo-tasks|migrate_plan_tasks_to_todo/.test(src)) {
        offenders.push(relative(repoRoot, file));
      }
      if (/todo_tasks.*plan_tasks|plan_tasks.*from.*todo_tasks/.test(src) && /rename|migrat/i.test(src)) {
        // Narrow: reverse rename of roots would be a falsifier.
        if (/todo_tasks["'].*plan_tasks["']|["']todo_tasks["'].*["']plan_tasks["']/.test(src)) {
          offenders.push(`${relative(repoRoot, file)}:possible-reverse-root`);
        }
      }
    }
    expect(offenders, `auto/reverse migrate refs:\n${offenders.join('\n')}`).toEqual([]);
  });

  it('missing .migration_gate_passed gates todo API only — ensure_todo_api_ungated, not App abort', () => {
    const service = read('src-tauri/src/services/todo_task/mod.rs');
    expect(service).toContain('ensure_todo_api_ungated');
    expect(service).toContain('.migration_gate_passed');
    expect(service).toMatch(/_status["']?\s*:\s*503|503/);
    // Must not process::exit / panic on missing gate.
    const gateFn = service.slice(
      service.indexOf('pub fn ensure_todo_api_ungated'),
      service.indexOf('pub fn ensure_todo_api_ungated') + 400,
    );
    expect(gateFn).not.toMatch(/process::exit|std::process::exit|panic!\(/);

    const http = read('src-tauri/src/services/local_http/mod.rs');
    expect(http).toContain('ensure_todo_api_ungated');
    expect(http).toMatch(/no auto-migrate|Durable migration gate/i);
  });
});

describe('T11 — npm wiring', () => {
  it('npm test includes this recovery-drill gate file', () => {
    const pkg = JSON.parse(read('package.json'));
    expect(pkg.scripts.test).toContain('tests/todo-task-recovery-drill.test.js');
  });
});
