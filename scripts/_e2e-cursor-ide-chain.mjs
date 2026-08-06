#!/usr/bin/env node
/**
 * Live chain smoke: Cursor-IDE slot → Host MCP → Sidecar HTTP → Rust domain.
 * Requires Host App up (127.0.0.1:9876 MCP + :8765 Sidecar).
 */
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.join(__dirname, '..');
const require = createRequire(path.join(REPO, 'archive/knowledge-mcp/package.json'));

const { Client } = await import(
  pathToFileURL(
    path.join(REPO, 'archive/knowledge-mcp/node_modules/@modelcontextprotocol/sdk/dist/esm/client/index.js'),
  ).href
);
const { StreamableHTTPClientTransport } = await import(
  pathToFileURL(
    path.join(
      REPO,
      'archive/knowledge-mcp/node_modules/@modelcontextprotocol/sdk/dist/esm/client/streamableHttp.js',
    ),
  ).href
);

const MCP_PORT = Number(process.env.MCP_PORT || 9876);
const HTTP_PORT = Number(process.env.WORKBENCH_HTTP_PORT || 8765);
const SLOT = 'cursor_ide';
const mcpUrl = `http://127.0.0.1:${MCP_PORT}/mcp/${SLOT}`;
const httpBase = `http://127.0.0.1:${HTTP_PORT}`;

function step(name, ok, detail = '') {
  const mark = ok ? 'PASS' : 'FAIL';
  console.log(`[${mark}] ${name}${detail ? ' — ' + detail : ''}`);
  if (!ok) throw new Error(name + (detail ? ': ' + detail : ''));
}

async function main() {
  console.log('=== cursor_ide chain e2e ===');
  console.log(`MCP:  ${mcpUrl}`);
  console.log(`HTTP: ${httpBase}/api/*`);

  // 1) Sidecar HTTP (Rust) alive
  const statusRes = await fetch(`${httpBase}/api/status`);
  const statusBody = await statusRes.text();
  step('HTTP Sidecar /api/status', statusRes.ok, `HTTP ${statusRes.status} ${statusBody.slice(0, 120)}`);

  // 2) Host MCP /health
  const healthRes = await fetch(`http://127.0.0.1:${MCP_PORT}/health`);
  const healthJson = await healthRes.json();
  step(
    'Host MCP GET /health',
    healthRes.ok && healthJson.ok === true && !!healthJson.mcp,
    JSON.stringify(healthJson),
  );

  // 3) MCP client initialize + tools/list on cursor_ide
  const transport = new StreamableHTTPClientTransport(new URL(mcpUrl));
  const client = new Client({ name: 'e2e-cursor-ide-chain', version: '0.1.0' });
  await client.connect(transport);
  step('MCP initialize (cursor_ide)', true, 'session established');

  const listed = await client.listTools();
  const names = (listed.tools || []).map((t) => t.name).sort();
  const need = ['get_corpus_catalog', 'list_todo_tasks', 'create_todo_task'];
  const missing = need.filter((n) => !names.includes(n));
  step(
    'MCP tools/list includes corpus+todo',
    missing.length === 0,
    `tools=${names.length}; missing=${missing.join(',') || 'none'}`,
  );

  // 4) Representative tools/call → Sidecar HTTP → Rust
  const call = await client.callTool({ name: 'list_todo_tasks', arguments: {} });
  const text =
    Array.isArray(call.content) && call.content[0]?.type === 'text'
      ? call.content[0].text
      : JSON.stringify(call);
  const okCall = !call.isError && (text.includes('"ok"') || text.includes('[') || text.includes('{'));
  step('MCP tools/call list_todo_tasks → HTTP → Rust', okCall, text.slice(0, 200));

  // 5) Dual-face check: unknown slot hard reject
  const unknown = await fetch(`http://127.0.0.1:${MCP_PORT}/mcp/__unknown__`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json, text/event-stream',
    },
    body: JSON.stringify({
      jsonrpc: '2.0',
      id: 1,
      method: 'initialize',
      params: {
        protocolVersion: '2025-03-26',
        capabilities: {},
        clientInfo: { name: 'e2e', version: '0' },
      },
    }),
  });
  step(
    'Unknown slot HTTP hard-reject (no session)',
    unknown.status === 404 && !unknown.headers.get('mcp-session-id'),
    `status=${unknown.status}`,
  );

  await client.close();
  console.log('=== ALL PASS: cursor IDE slot → MCP → HTTP → Rust ===');
}

main().catch((err) => {
  console.error('=== FAILED ===');
  console.error(err);
  process.exit(1);
});
