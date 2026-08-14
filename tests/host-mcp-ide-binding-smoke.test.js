/**
 * T11 / P3 — IDE/Binding acceptance smoke gate on Host MCP URL.
 * Checklist is the deliverable (tdd_exempt); this test only gates the record.
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const CHECKLIST_PATH = join(
  repoRoot,
  'docs/archive/mcp/host-mcp-native-adapter/ide-binding-acceptance-smoke.md',
);
const HOST_MCP_SLOT_URL = 'http://127.0.0.1:9876/mcp/';

const REQUIRED_MARKERS = [
  HOST_MCP_SLOT_URL,
  'cursor_ide',
  'todo_task',
  'Host Agent Loop',
  'registry',
  'IDE confirmation: BLOCKED',
  'AC: NOT_PASSED',
];

describe('T11 IDE/Binding Host MCP acceptance smoke record', () => {
  it('docs checklist exists with Host URL dual-slot + BLOCKED IDE gate (F-28)', () => {
    expect(
      existsSync(CHECKLIST_PATH),
      'missing docs/archive/mcp/host-mcp-native-adapter/ide-binding-acceptance-smoke.md',
    ).toBe(true);
    const doc = readFileSync(CHECKLIST_PATH, 'utf8');
    for (const marker of REQUIRED_MARKERS) {
      expect(doc, `checklist must contain marker: ${marker}`).toContain(marker);
    }
    // Must not claim full AC pass while IDE confirmation is blocked.
    expect(doc).not.toMatch(/AC:\s*PASSED/);
    expect(doc).toMatch(/Host Agent Loop/);
  });

  it('npm test includes T11 IDE/Binding smoke gate', () => {
    const pkg = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8'));
    expect(pkg.scripts.test).toContain('tests/host-mcp-ide-binding-smoke.test.js');
  });

  it('Cursor Agent runner is removed from the Host MCP surface', () => {
    const pkgJson = join(repoRoot, 'packages/cursor-agent-runner/package.json');
    expect(existsSync(pkgJson), 'cursor-agent-runner package must be removed').toBe(false);
    const adapter = readFileSync(
      join(repoRoot, 'src-tauri/src/services/mcp_protocol_adapter.rs'),
      'utf8',
    );
    expect(adapter).not.toMatch(/cursor-agent-runner/);
    const libRs = readFileSync(join(repoRoot, 'src-tauri/src/lib.rs'), 'utf8');
    expect(libRs).not.toMatch(/packages\/cursor-agent-runner/);
  });
});
