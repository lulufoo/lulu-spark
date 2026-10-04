/**
 * T10 / P3 — archive Node knowledge-mcp; retarget contract tests to Host MCP URL.
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const HOST_MCP_BASE = 'http://127.0.0.1:9876';

function read(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

describe('T10 Host MCP archive + contract retarget (V2/V3/V4)', () => {
  it('packages/knowledge-mcp/index.mjs is not a runtime SSOT (archived/removed)', () => {
    expect(
      existsSync(join(repoRoot, 'packages/knowledge-mcp/index.mjs')),
      'T10/V4: packages/knowledge-mcp/index.mjs must not remain as runtime SSOT',
    ).toBe(false);
    expect(
      existsSync(join(repoRoot, 'packages/knowledge-mcp/package.json')),
      'T10/V4: packages/knowledge-mcp must not remain installable as a runtime package',
    ).toBe(false);
    expect(
      existsSync(join(repoRoot, 'archive/knowledge-mcp/index.mjs')),
      'archive/knowledge-mcp must not remain as a Node MCP snapshot',
    ).toBe(false);
    expect(
      existsSync(join(repoRoot, 'archive/knowledge-mcp/package.json')),
      'archive/knowledge-mcp must not remain installable as a Node package',
    ).toBe(false);
  });

  it('npm test no longer installs or runs Node packages/knowledge-mcp', () => {
    const pkg = JSON.parse(read('package.json'));
    const testScript = pkg.scripts.test;
    expect(testScript).not.toMatch(/npm install --prefix packages\/knowledge-mcp/);
    expect(testScript).not.toContain('packages/knowledge-mcp/scripts/verify.mjs');
    expect(testScript).not.toContain('packages/knowledge-mcp/index.mjs');
  });

  it('verify-host-mcp contract script targets Host MCP URL http://127.0.0.1:9876', () => {
    const verifyPath = join(repoRoot, 'scripts/verify-host-mcp.mjs');
    expect(existsSync(verifyPath), 'missing scripts/verify-host-mcp.mjs').toBe(true);
    const src = readFileSync(verifyPath, 'utf8');
    expect(src).toContain(HOST_MCP_BASE);
    expect(src).toMatch(/spark/);
    expect(src).toMatch(/cursor_ide/);
    // Must not spawn Node MCP as a process (path may appear only as archive-gate joins)
    expect(src).not.toMatch(/spawnSidecar\s*\(/);
    expect(src).not.toMatch(/spawn\([^)]*index\.mjs/);
    expect(src).not.toMatch(/['"]packages\/knowledge-mcp\/index\.mjs['"]/);
  });

  it('verify-host-mcp registers spark + cursor_ide, not todo_task as App slot', () => {
    const src = read('scripts/verify-host-mcp.mjs');
    expect(src).toContain("REGISTERED_SLOTS = ['spark', 'cursor_ide']");
    expect(src).not.toMatch(
      /REGISTERED_SLOTS\s*=\s*\[[^\]]*(['"])todo_task\1/,
    );
    expect(src).not.toMatch(/REGISTERED_SLOTS\s*=\s*\[[^\]]*(['"])notes\1/);
    expect(src).not.toContain('get_notes_selection');
  });

  it('verify-host-mcp asserts retired App slots /mcp/notes and /mcp/todo_task HTTP 404', () => {
    const src = read('scripts/verify-host-mcp.mjs');
    expect(src).toMatch(/\/mcp\/notes/);
    expect(src).toMatch(/\/mcp\/todo_task/);
    expect(src).toMatch(
      /RETIRED_APP_SLOTS\s*=\s*\[[^\]]*(['"])notes\1[^\]]*(['"])todo_task\2/,
    );
    expect(src).toMatch(/status !== 404|status === 404/);
  });

  it('todo-task-mcp-e2e reads architecture constraints for Skills loopback and L0 paths', () => {
    const src = read('scripts/todo-task-mcp-e2e.mjs');
    expect(src).toContain('docs/architecture/arch-layer-constraints.md');
    expect(src).toContain('127.0.0.1:9876/mcp/cursor_ide');
    expect(src).toContain('127.0.0.1:9876/mcp/spark');
    expect(src).toMatch(/\/mcp\/spark/);
    expect(src).not.toContain('docs/knowledge-mcp.md');
    expect(src).not.toContain('frontend/js/todo-task/binding.js');
    expect(src).not.toContain('http://127.0.0.1:<mcp_port>/mcp/todo_task');
    expect(src).not.toMatch(
      /new URL\(`http:\/\/127\.0\.0\.1:\$\{mcpPort\}\/mcp\/todo_task`\)/,
    );
    expect(src).not.toMatch(/listToolNamesOnSlot\(mcpPort,\s*['"]todo_task['"]/);
    expect(src).toMatch(/listToolNamesOnSlot\(mcpPort,\s*['"]spark['"]/);
    expect(src).toMatch(/must not expose get_notes_selection/);
  });

  it('live Host MCP URLs keep App spark + IDE cursor_ide', () => {
    const verify = read('scripts/verify-host-mcp.mjs');
    expect(verify).toContain("REGISTERED_SLOTS = ['spark', 'cursor_ide']");
    expect(verify).not.toContain('get_notes_selection');
    expect(verify).not.toMatch(/http:\/\/127\.0\.0\.1:<mcp_port>\/#\/spark/);
  });

  it('P4 smoke checklist P6 no longer writes off-page Reset', () => {
    const checklist = read('tests/gates/ai-assistant-p4-smoke-checklist.md');
    expect(checklist).toContain('P6：离页不 Reset');
    expect(checklist).not.toMatch(/P6：离页 Reset/);
  });

  it('Host MCP verify script is a manual entry, not part of npm test', () => {
    const pkg = JSON.parse(read('package.json'));
    expect(pkg.scripts.test).not.toContain('verify-host-mcp');
    expect(pkg.scripts['verify:host-mcp']).toBe('node scripts/verify-host-mcp.mjs');
  });

  it('archived Node knowledge-mcp snapshot is removed', () => {
    expect(
      existsSync(join(repoRoot, 'archive/knowledge-mcp')),
      'archive/knowledge-mcp must be gone',
    ).toBe(false);
  });

  it('e2e contract entry defaults to Host MCP URL :9876', () => {
    const e2ePath = join(repoRoot, 'scripts/todo-task-mcp-e2e.mjs');
    expect(existsSync(e2ePath), 'missing scripts/todo-task-mcp-e2e.mjs').toBe(true);
    const src = readFileSync(e2ePath, 'utf8');
    expect(src).toMatch(/127\.0\.0\.1:\$\{mcpPort\}|127\.0\.0\.1:9876|HOST_MCP/);
    // Default port when env unset should be Host 9876
    expect(src).toMatch(/9876/);
    expect(src).not.toMatch(/['"]packages\/knowledge-mcp\/index\.mjs['"]/);
  });

  it('Sidecar HTTP fixture module is independent of MCP process model', () => {
    const fixturePath = join(repoRoot, 'scripts/sidecar-http-fixture.mjs');
    expect(existsSync(fixturePath), 'missing scripts/sidecar-http-fixture.mjs').toBe(true);
    const src = readFileSync(fixturePath, 'utf8');
    // Fixture may start mock HTTP; must not spawn MCP (Node or Host).
    expect(src).not.toMatch(/spawn\s*\(/);
    expect(src).not.toMatch(/index\.mjs/);
    expect(src).not.toMatch(/start_embedded_mcp|MCP_PORT/);
    expect(src).toMatch(/createServer|http\.createServer|listen/);
  });
});
