#!/usr/bin/env node
/**
 * knowledge-mcp verification: mock Workbench HTTP + sidecar MCP client (TDD / CI).
 */
import { spawn } from 'node:child_process';
import http from 'node:http';
import { createServer } from 'node:net';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PKG_ROOT = path.join(__dirname, '..');

const INDEX = {
  layers: { digest: { 'demo-topic': { latest: 'demo-topic/sample.md' } } },
};

const FILE_CONTENT = {
  content: '# Demo digest\n\nknowledge-mcp mock digest body.\n',
};

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function ephemeralPort() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
    server.on('error', reject);
  });
}

async function waitFor(url, attempts = 40) {
  for (let i = 0; i < attempts; i++) {
    try {
      const res = await fetch(url);
      if (res.ok) return true;
    } catch {
      /* retry */
    }
    await sleep(150);
  }
  return false;
}

function startMockHttp(port) {
  const server = http.createServer((req, res) => {
    const url = new URL(req.url || '/', `http://127.0.0.1:${port}`);
    if (req.method !== 'GET') {
      res.writeHead(405).end(JSON.stringify({ error: 'method not allowed' }));
      return;
    }
    if (url.pathname === '/api/corpus-index') {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(INDEX));
      return;
    }
    if (url.pathname === '/api/corpus-file') {
      const layer = url.searchParams.get('layer') || '';
      const filePath = url.searchParams.get('path') || '';
      if (layer !== 'digest') {
        res.writeHead(400).end(JSON.stringify({ error: `Invalid layer: ${layer}` }));
        return;
      }
      if (filePath !== 'demo-topic/sample.md') {
        res.writeHead(404).end(JSON.stringify({ error: 'File not found' }));
        return;
      }
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(FILE_CONTENT));
      return;
    }
    res.writeHead(404).end(JSON.stringify({ error: 'not found' }));
  });

  return new Promise((resolve, reject) => {
    server.listen(port, '127.0.0.1', () => resolve(server));
    server.on('error', reject);
  });
}

function spawnSidecar(mcpPort, workbenchUrl, extraEnv = {}) {
  const child = spawn(process.execPath, [path.join(PKG_ROOT, 'index.mjs')], {
    stdio: ['ignore', 'pipe', 'pipe'],
    env: {
      ...process.env,
      MCP_PORT: String(mcpPort),
      WORKBENCH_HTTP_URL: workbenchUrl,
      ...extraEnv,
    },
  });
  child.stderr.on('data', (chunk) => process.stderr.write(chunk));
  return child;
}

async function runMcpClient(mcpPort) {
  const mcpUrl = `http://127.0.0.1:${mcpPort}/mcp`;
  const transport = new StreamableHTTPClientTransport(new URL(mcpUrl));
  const client = new Client({ name: 'knowledge-mcp-verify', version: '0.1.0' });
  await client.connect(transport);

  const tools = await client.listTools();
  const names = tools.tools.map((t) => t.name);
  if (!names.includes('get_corpus_index') || !names.includes('get_corpus_file')) {
    throw new Error(`missing tools: ${names.join(', ')}`);
  }

  const indexResult = await client.callTool({ name: 'get_corpus_index', arguments: {} });
  const indexText = indexResult.content?.[0]?.text || '';
  if (!indexText.includes('demo-topic')) {
    throw new Error(`unexpected index: ${indexText}`);
  }

  const fileResult = await client.callTool({
    name: 'get_corpus_file',
    arguments: { path: 'demo-topic/sample.md' },
  });
  const fileText = fileResult.content?.[0]?.text || '';
  if (!fileText.includes('knowledge-mcp mock digest')) {
    throw new Error(`unexpected file: ${fileText}`);
  }

  const missingFile = await client.callTool({
    name: 'get_corpus_file',
    arguments: { path: 'missing.md' },
  });
  const missingText = missingFile.content?.[0]?.text || '';
  if (!missingFile.isError || !missingText.includes('HTTP 404')) {
    throw new Error(`expected HTTP 404 tool error, got: ${missingText}`);
  }

  await client.close();
}

async function testMissingWorkbenchUrl() {
  const mcpPort = await ephemeralPort();
  const noUrlChild = spawn(process.execPath, [path.join(PKG_ROOT, 'index.mjs')], {
    stdio: ['ignore', 'pipe', 'pipe'],
    env: Object.fromEntries(
      Object.entries({ ...process.env, MCP_PORT: String(mcpPort) }).filter(
        ([key]) => key !== 'WORKBENCH_HTTP_URL',
      ),
    ),
  });
  const stderrChunks = [];
  noUrlChild.stderr.on('data', (chunk) => stderrChunks.push(chunk.toString()));
  const exitCode = await new Promise((resolve) => {
    noUrlChild.on('exit', (code) => resolve(code ?? 1));
  });
  const stderr = stderrChunks.join('');
  if (exitCode === 0) {
    throw new Error('expected exit when WORKBENCH_HTTP_URL is missing');
  }
  if (!stderr.includes('WORKBENCH_HTTP_URL')) {
    throw new Error(`stderr missing WORKBENCH_HTTP_URL error: ${stderr}`);
  }
}

async function main() {
  const httpPort = await ephemeralPort();
  const mcpPort = await ephemeralPort();
  const workbenchUrl = `http://127.0.0.1:${httpPort}`;

  const mockServer = await startMockHttp(httpPort);
  const sidecar = spawnSidecar(mcpPort, workbenchUrl);

  const ready = await waitFor(`http://127.0.0.1:${mcpPort}/health`);
  if (!ready) {
    sidecar.kill('SIGTERM');
    mockServer.close();
    throw new Error('sidecar health check timeout');
  }

  await runMcpClient(mcpPort);
  console.log('MCP proxy tools: OK');

  sidecar.kill('SIGTERM');
  mockServer.close();
  await sleep(300);

  await testMissingWorkbenchUrl();
  console.log('missing WORKBENCH_HTTP_URL exits: OK');

  console.log('\nknowledge-mcp verify PASSED');
}

main().catch((err) => {
  console.error('knowledge-mcp verify FAILED:', err);
  process.exit(1);
});
