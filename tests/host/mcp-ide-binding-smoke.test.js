/**
 * T11 / P3 — IDE/Binding acceptance smoke on live Host MCP surface.
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { readRsPath } from '../helpers/read-rs-dir.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

describe('T11 IDE/Binding Host MCP acceptance smoke record', () => {
  it('Host registers workbench + cursor_ide; README documents the IDE URL', () => {
    const readme = readFileSync(join(repoRoot, 'README.md'), 'utf8');
    const types = readFileSync(
      join(repoRoot, 'src-tauri/src/services/mcp_protocol_adapter/types.rs'),
      'utf8',
    );
    expect(readme).toContain('http://127.0.0.1:<mcp_port>/mcp/cursor_ide');
    expect(types).toContain('cursor_ide');
    expect(types).toContain('workbench');
  });

  it('npm test includes T11 IDE/Binding smoke gate', () => {
    const pkg = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8'));
    expect(pkg.scripts.test).toMatch(/vitest run --dir tests/);
  });

  it('Cursor Agent runner is removed from the Host MCP surface', () => {
    const pkgJson = join(repoRoot, 'packages/cursor-agent-runner/package.json');
    expect(existsSync(pkgJson), 'cursor-agent-runner package must be removed').toBe(false);
    const adapter = readRsPath(
      join(repoRoot, 'src-tauri/src/services/mcp_protocol_adapter'),
    );
    expect(adapter).not.toMatch(/cursor-agent-runner/);
    const libRs = readFileSync(join(repoRoot, 'src-tauri/src/lib.rs'), 'utf8');
    expect(libRs).not.toMatch(/packages\/cursor-agent-runner/);
  });
});
