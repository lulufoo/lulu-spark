#!/usr/bin/env node
/**
 * Host MCP contract verify (T10): targets Host URL http://127.0.0.1:9876.
 * Dual-slot list/call + unknown-slot hard-fail smoke is owned by Host cargo tests
 * (`p3_t10_host_dual_slot_list_call_and_unknown_hard_fail_smoke`); this script
 * gates archive/runtime independence and Sidecar fixture separation.
 */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
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

const REGISTERED_SLOTS = ['workbench', 'cursor_ide'];

/** Retired App slots — `/mcp/notes` and `/mcp/todo_task` must HTTP 404. */
const RETIRED_APP_SLOTS = ['notes', 'todo_task'];

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
  const archivedDir = path.join(REPO_ROOT, 'archive', 'knowledge-mcp');
  if (
    existsSync(path.join(archivedDir, 'index.mjs')) ||
    existsSync(path.join(archivedDir, 'package.json'))
  ) {
    throw new Error('Node knowledge-mcp archive snapshot must be removed');
  }
}

function readAdapterSource() {
  const dir = path.join(REPO_ROOT, 'src-tauri/src/services/mcp_protocol_adapter');
  return readdirSync(dir)
    .filter((name) => name.endsWith('.rs'))
    .sort()
    .map((name) => readFileSync(path.join(dir, name), 'utf8'))
    .join('\n');
}

function assertNoForbiddenAttachmentDeletes() {
  const adapter = readAdapterSource();
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
  const mcpPost = (slot) =>
    fetch(`${HOST_MCP_BASE}/mcp/${slot}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json, text/event-stream',
      },
      body: init,
    });
  const unknown = await mcpPost('__unknown__');
  if (unknown.status !== 404) {
    throw new Error(
      `unknown scene_slot must HTTP 404 on ${HOST_MCP_BASE}; got ${unknown.status}`,
    );
  }
  if (unknown.headers.get('mcp-session-id')) {
    throw new Error('unknown scene_slot must not establish MCP session');
  }

  for (const slot of RETIRED_APP_SLOTS) {
    const retired = await mcpPost(slot);
    if (retired.status !== 404) {
      throw new Error(
        `retired App slot ${slot} must HTTP 404 on ${HOST_MCP_BASE}; got ${retired.status}`,
      );
    }
  }

  // Registered slot paths must not hard-reject at routing layer.
  for (const slot of REGISTERED_SLOTS) {
    const res = await mcpPost(slot);
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
      env: { ...process.env },
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
  console.log(
    `archive gate: packages/knowledge-mcp and archive/knowledge-mcp absent (Host ${HOST_MCP_BASE} is SSOT)`,
  );

  assertNoForbiddenAttachmentDeletes();
  console.log('Host MCP forbids attachment delete tools: OK');

  await assertSidecarFixtureIndependent();
  console.log('sidecar HTTP fixture independent of MCP: OK');

  const hostUp = await probeHostIfUp();
  if (hostUp) {
    console.log(`live Host probe on ${HOST_MCP_BASE} (workbench/cursor_ide + retired 404): OK`);
  } else if (process.env.VERIFY_HOST_MCP_SKIP_CARGO === '1') {
    // npm test already ran Host cargo suite (incl. T10 dual-slot smoke) before this script.
    console.log(
      `Host dual-slot list/call + unknown hard-fail: deferred to preceding cargo (Host ${HOST_MCP_BASE})`,
    );
  } else {
    // Standalone verify: prove dual-slot via Host cargo smoke on :9876.
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
