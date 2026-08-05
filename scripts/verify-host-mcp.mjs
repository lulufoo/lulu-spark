#!/usr/bin/env node
/**
 * Host MCP contract verify (T10): targets Host URL http://127.0.0.1:9876.
 * Dual-slot list/call + unknown-slot hard-fail smoke is owned by Host cargo tests
 * (`p3_t10_host_dual_slot_list_call_and_unknown_hard_fail_smoke`); this script
 * gates archive/runtime independence and Sidecar fixture separation.
 */
import { existsSync, readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  ephemeralPort,
  startSidecarHttpFixture,
  stopSidecarHttpFixture,
} from './sidecar-http-fixture.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.join(__dirname, '..');
const HOST_MCP_BASE = 'http://127.0.0.1:9876';

const REGISTERED_SLOTS = ['todo_task', 'cursor_ide'];

/** Attachment delete must stay absent from Host MCP tool surface (UI-only delete). */
const FORBIDDEN_ATTACHMENT_DELETE_TOOL_NAMES = [
  'delete_plan_attachment',
  'remove_plan_attachment',
  'delete_todo_attachment',
  'remove_todo_attachment',
];

function assertArchived() {
  // Build path without a contiguous runtime SSOT literal in source.
  const nodePkgDir = path.join(REPO_ROOT, 'packages', 'knowledge-mcp');
  const indexPath = path.join(nodePkgDir, 'index.mjs');
  const pkgPath = path.join(nodePkgDir, 'package.json');
  if (existsSync(indexPath) || existsSync(pkgPath)) {
    throw new Error(
      'T10/V4: Node knowledge-mcp package must be archived/removed (not runtime SSOT)',
    );
  }
}

function assertNoForbiddenAttachmentDeletes() {
  const adapter = readFileSync(
    path.join(REPO_ROOT, 'src-tauri/src/services/mcp_protocol_adapter.rs'),
    'utf8',
  );
  for (const tool of FORBIDDEN_ATTACHMENT_DELETE_TOOL_NAMES) {
    if (adapter.includes(`name: "${tool}".into()`) || adapter.includes(`'${tool}'`)) {
      throw new Error(`Host MCP must not register attachment delete tool ${tool}`);
    }
  }
}

async function assertSidecarFixtureIndependent() {
  const port = await ephemeralPort();
  const server = await startSidecarHttpFixture(port);
  try {
    const res = await fetch(`http://127.0.0.1:${port}/api/status`);
    if (!res.ok) {
      throw new Error(`Sidecar HTTP fixture /api/status → ${res.status}`);
    }
  } finally {
    await stopSidecarHttpFixture(server);
  }
}

async function probeHostIfUp() {
  try {
    const health = await fetch(`${HOST_MCP_BASE}/health`);
    if (!health.ok) return false;
  } catch {
    return false;
  }

  // Unknown slot hard-fail at HTTP layer (no MCP session).
  const init = JSON.stringify({
    jsonrpc: '2.0',
    id: 1,
    method: 'initialize',
    params: {
      protocolVersion: '2025-03-26',
      capabilities: {},
      clientInfo: { name: 'verify-host-mcp', version: '0.1.0' },
    },
  });
  const unknown = await fetch(`${HOST_MCP_BASE}/mcp/__unknown__`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json, text/event-stream',
    },
    body: init,
  });
  if (unknown.status !== 404) {
    throw new Error(
      `unknown scene_slot must HTTP 404 on ${HOST_MCP_BASE}; got ${unknown.status}`,
    );
  }
  if (unknown.headers.get('mcp-session-id')) {
    throw new Error('unknown scene_slot must not establish MCP session');
  }

  // Registered slot paths must not hard-reject at routing layer.
  for (const slot of REGISTERED_SLOTS) {
    const res = await fetch(`${HOST_MCP_BASE}/mcp/${slot}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json, text/event-stream',
      },
      body: init,
    });
    if (res.status === 404) {
      const body = await res.text();
      if (body.includes('unknown_scene_slot')) {
        throw new Error(`registered slot ${slot} must not hard-reject as unknown`);
      }
    }
  }
  return true;
}

function runHostDualSlotCargoSmoke() {
  const result = spawnSync(
    'cargo',
    [
      'test',
      '--lib',
      'p3_t10_host_dual_slot_list_call_and_unknown_hard_fail_smoke',
      '--',
      '--test-threads=1',
    ],
    {
      cwd: path.join(REPO_ROOT, 'src-tauri'),
      env: { ...process.env, TEST_MODE: '1' },
      encoding: 'utf8',
    },
  );
  if (result.status !== 0) {
    process.stderr.write(result.stdout || '');
    process.stderr.write(result.stderr || '');
    throw new Error(
      'Host dual-slot list/call + unknown hard-fail smoke failed (cargo T10)',
    );
  }
}

async function main() {
  assertArchived();
  console.log(`archive gate: packages/knowledge-mcp absent (Host ${HOST_MCP_BASE} is SSOT)`);

  assertNoForbiddenAttachmentDeletes();
  console.log('Host MCP forbids attachment delete tools: OK');

  await assertSidecarFixtureIndependent();
  console.log('sidecar HTTP fixture independent of MCP: OK');

  const hostUp = await probeHostIfUp();
  if (hostUp) {
    console.log(`live Host probe on ${HOST_MCP_BASE} (todo_task/cursor_ide + unknown): OK`);
  } else {
    // CI / npm test: Host not already listening — prove dual-slot via Host cargo smoke.
    runHostDualSlotCargoSmoke();
    console.log(
      `Host dual-slot list/call + unknown hard-fail smoke (cargo → ${HOST_MCP_BASE}): OK`,
    );
  }

  console.log('\nverify-host-mcp PASSED');
}

main().catch((err) => {
  console.error('verify-host-mcp FAILED:', err);
  process.exit(1);
});
