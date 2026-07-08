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

const DEMO_ID = '138700959e5ddb1260c69e9e18169ac4';
const CATALOG = {
  items: [
    { id: DEMO_ID, topic: 'demo-topic', created_at: '202606190004' },
  ],
};

const FILE_BATCH = {
  items: [
    { id: DEMO_ID, ok: true, content: '# Demo digest\n\nknowledge-mcp mock digest body.\n' },
  ],
};

const ARCHIVE_DOC_RESPONSE = {
  ok: true,
  id: DEMO_ID,
  common_path: 'demo-topic/note.md',
  raw_path: 'raw/demo-topic/note.md',
  created_at: '202606190004',
};

const ARCHIVE_DIGEST_RESPONSE = {
  ok: true,
  id: DEMO_ID,
  common_path: 'demo-topic/note.md',
  digest_path: 'digest/demo-topic/note.md',
};

const PLAN_TASK_CREATE_RESPONSE = {
  master_task_id: 'task_mock001',
  sub_task_id: 'task_mock001_sub_01',
  task: {
    master_task_id: 'task_mock001',
    title: 'Mock master',
    status: 'incomplete',
    created_at: '2026-07-07T00:00:00Z',
    sub_tasks: [
      {
        sub_task_id: 'task_mock001_sub_01',
        title: 'Mock master',
        status: 'incomplete',
        implicit: true,
        linked_archive_ids: [],
      },
    ],
  },
};

/** @type {Array<{ title: string, sub_titles?: string[] }>} */
const planTaskCreateCalls = [];

const PLAN_TOOL_NAMES = [
  'create_plan_task',
  'list_plan_tasks',
  'get_plan_task',
  'delete_plan_task',
  'add_plan_sub',
  'delete_plan_sub',
  'complete_plan_sub',
  'link_plan_archive',
];

/** @type {Map<string, object>} */
const planTaskStore = new Map();
let planSubSeq = 0;

function resetPlanTaskStore() {
  planTaskStore.clear();
  planSubSeq = 0;
}

function nextSubId(masterId) {
  planSubSeq += 1;
  return `${masterId}_sub_${String(planSubSeq).padStart(2, '0')}`;
}

function planTaskSnapshot() {
  return Array.from(planTaskStore.values());
}

function respondJson(res, status, body) {
  res.writeHead(status, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify(body));
}

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

function readJsonBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    req.on('data', (chunk) => chunks.push(chunk));
    req.on('end', () => {
      try {
        const text = Buffer.concat(chunks).toString('utf8');
        resolve(text ? JSON.parse(text) : {});
      } catch (err) {
        reject(err);
      }
    });
    req.on('error', reject);
  });
}

function startMockHttp(port) {
  const server = http.createServer(async (req, res) => {
    const url = new URL(req.url || '/', `http://127.0.0.1:${port}`);

    if (req.method === 'GET' && url.pathname === '/api/corpus-catalog') {
      const mode = url.searchParams.get('mode') || '';
      if (mode !== 'latest_per_topic') {
        res.writeHead(400).end(JSON.stringify({ error: 'Unsupported mode' }));
        return;
      }
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(CATALOG));
      return;
    }

    if (req.method === 'POST' && url.pathname === '/api/corpus-files') {
      let payload;
      try {
        payload = await readJsonBody(req);
      } catch {
        res.writeHead(400).end(JSON.stringify({ error: 'Invalid JSON' }));
        return;
      }
      const ids = payload.ids || [];
      if (!Array.isArray(ids) || ids.length === 0) {
        res.writeHead(400).end(JSON.stringify({ error: 'Missing ids array' }));
        return;
      }
      if (ids.includes('missing-id')) {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
          items: [{ id: 'missing-id', ok: false, error: 'Entry not found' }],
        }));
        return;
      }
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(FILE_BATCH));
      return;
    }

    if (req.method === 'POST' && url.pathname === '/api/archive-document') {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(ARCHIVE_DOC_RESPONSE));
      return;
    }

    if (req.method === 'POST' && url.pathname === '/api/archive-digest') {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(ARCHIVE_DIGEST_RESPONSE));
      return;
    }

    if (req.method === 'GET' && url.pathname === '/api/plan-tasks') {
      respondJson(res, 200, planTaskSnapshot());
      return;
    }

    if (req.method === 'GET' && url.pathname === '/api/plan-task') {
      const id = (url.searchParams.get('id') || '').trim();
      if (!id) {
        respondJson(res, 400, { error: 'Missing id' });
        return;
      }
      const task = planTaskStore.get(id);
      if (!task) {
        respondJson(res, 404, { error: 'Task not found' });
        return;
      }
      respondJson(res, 200, task);
      return;
    }

    if (req.method === 'POST' && url.pathname === '/api/plan-task-create') {
      let payload;
      try {
        payload = await readJsonBody(req);
      } catch {
        respondJson(res, 400, { error: 'Invalid JSON' });
        return;
      }
      const title = typeof payload.title === 'string' ? payload.title.trim() : '';
      if (!title) {
        respondJson(res, 400, { error: 'Missing title' });
        return;
      }
      const subTitles = Array.isArray(payload.sub_titles) ? payload.sub_titles : undefined;
      if (subTitles != null) {
        for (const item of subTitles) {
          if (typeof item !== 'string' || item.trim() === '') {
            respondJson(res, 400, { error: 'Invalid sub_titles element' });
            return;
          }
        }
      }
      planTaskCreateCalls.push({
        title,
        ...(subTitles != null ? { sub_titles: subTitles } : {}),
      });
      const masterId = 'task_mock001';
      const explicit = subTitles != null && subTitles.length > 0;
      const subs = explicit
        ? subTitles.map((t) => ({
            sub_task_id: nextSubId(masterId),
            title: t,
            status: 'incomplete',
            implicit: false,
            linked_archive_ids: [],
          }))
        : [
            {
              sub_task_id: nextSubId(masterId),
              title,
              status: 'incomplete',
              implicit: true,
              linked_archive_ids: [],
            },
          ];
      const task = {
        master_task_id: masterId,
        title,
        status: 'incomplete',
        created_at: '2026-07-07T00:00:00Z',
        sub_tasks: subs,
      };
      planTaskStore.set(masterId, task);
      respondJson(res, 201, {
        master_task_id: masterId,
        sub_task_id: subs[0].sub_task_id,
        task,
      });
      return;
    }

    if (req.method === 'POST' && url.pathname === '/api/plan-task-delete') {
      let payload;
      try {
        payload = await readJsonBody(req);
      } catch {
        respondJson(res, 400, { error: 'Invalid JSON' });
        return;
      }
      const masterId = typeof payload.master_task_id === 'string' ? payload.master_task_id.trim() : '';
      if (!masterId) {
        respondJson(res, 400, { error: 'Missing master_task_id' });
        return;
      }
      if (!planTaskStore.has(masterId)) {
        respondJson(res, 404, { error: 'Task not found' });
        return;
      }
      planTaskStore.delete(masterId);
      respondJson(res, 200, { ok: true });
      return;
    }

    if (req.method === 'POST' && url.pathname === '/api/plan-task-add-sub') {
      let payload;
      try {
        payload = await readJsonBody(req);
      } catch {
        respondJson(res, 400, { error: 'Invalid JSON' });
        return;
      }
      const masterId = typeof payload.master_task_id === 'string' ? payload.master_task_id.trim() : '';
      const title = typeof payload.title === 'string' ? payload.title.trim() : '';
      if (!masterId) {
        respondJson(res, 400, { error: 'Missing master_task_id' });
        return;
      }
      if (!title) {
        respondJson(res, 400, { error: 'Missing title' });
        return;
      }
      const task = planTaskStore.get(masterId);
      if (!task) {
        respondJson(res, 404, { error: 'Task not found' });
        return;
      }
      const sub = {
        sub_task_id: nextSubId(masterId),
        title,
        status: 'incomplete',
        implicit: false,
        linked_archive_ids: [],
      };
      task.sub_tasks.push(sub);
      respondJson(res, 201, { sub_task_id: sub.sub_task_id, task });
      return;
    }

    if (req.method === 'POST' && url.pathname === '/api/plan-task-delete-sub') {
      let payload;
      try {
        payload = await readJsonBody(req);
      } catch {
        respondJson(res, 400, { error: 'Invalid JSON' });
        return;
      }
      const masterId = typeof payload.master_task_id === 'string' ? payload.master_task_id.trim() : '';
      const subId = typeof payload.sub_task_id === 'string' ? payload.sub_task_id.trim() : '';
      if (!masterId || !subId) {
        respondJson(res, 400, { error: 'Missing master_task_id or sub_task_id' });
        return;
      }
      const task = planTaskStore.get(masterId);
      if (!task) {
        respondJson(res, 404, { error: 'Task not found' });
        return;
      }
      if (task.sub_tasks.length <= 1) {
        respondJson(res, 400, { error: 'Cannot delete last sub task' });
        return;
      }
      task.sub_tasks = task.sub_tasks.filter((s) => s.sub_task_id !== subId);
      respondJson(res, 200, { task });
      return;
    }

    if (req.method === 'POST' && url.pathname === '/api/plan-task-complete-sub') {
      let payload;
      try {
        payload = await readJsonBody(req);
      } catch {
        respondJson(res, 400, { error: 'Invalid JSON' });
        return;
      }
      const masterId = typeof payload.master_task_id === 'string' ? payload.master_task_id.trim() : '';
      const subId = typeof payload.sub_task_id === 'string' ? payload.sub_task_id.trim() : '';
      if (!masterId || !subId) {
        respondJson(res, 400, { error: 'Missing master_task_id or sub_task_id' });
        return;
      }
      const task = planTaskStore.get(masterId);
      if (!task) {
        respondJson(res, 404, { error: 'Task not found' });
        return;
      }
      const sub = task.sub_tasks.find((s) => s.sub_task_id === subId);
      if (!sub) {
        respondJson(res, 404, { error: 'Sub task not found' });
        return;
      }
      sub.status = 'complete';
      respondJson(res, 200, { task });
      return;
    }

    if (req.method === 'POST' && url.pathname === '/api/plan-task-link-archive') {
      let payload;
      try {
        payload = await readJsonBody(req);
      } catch {
        respondJson(res, 400, { error: 'Invalid JSON' });
        return;
      }
      const masterId = typeof payload.master_task_id === 'string' ? payload.master_task_id.trim() : '';
      const subId = typeof payload.sub_task_id === 'string' ? payload.sub_task_id.trim() : '';
      const archiveId = typeof payload.archive_id === 'string' ? payload.archive_id.trim() : '';
      if (!masterId || !subId || !archiveId) {
        respondJson(res, 400, { error: 'Missing master_task_id, sub_task_id, or archive_id' });
        return;
      }
      const task = planTaskStore.get(masterId);
      if (!task) {
        respondJson(res, 404, { error: 'Task not found' });
        return;
      }
      const sub = task.sub_tasks.find((s) => s.sub_task_id === subId);
      if (!sub) {
        respondJson(res, 404, { error: 'Sub task not found' });
        return;
      }
      if (sub.status !== 'complete') {
        respondJson(res, 400, { error: 'Sub task must be complete before linking archive' });
        return;
      }
      if (!sub.linked_archive_ids.includes(archiveId)) {
        sub.linked_archive_ids.push(archiveId);
      }
      respondJson(res, 200, { task });
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
  if (!names.includes('get_corpus_catalog') || !names.includes('get_corpus_files')) {
    throw new Error(`missing tools: ${names.join(', ')}`);
  }
  if (!names.includes('archive_document') || !names.includes('archive_digest')) {
    throw new Error(`missing archive tools: ${names.join(', ')}`);
  }
  if (!names.includes('create_plan_task')) {
    throw new Error(`missing create_plan_task tool: ${names.join(', ')}`);
  }
  for (const tool of PLAN_TOOL_NAMES) {
    if (!names.includes(tool)) {
      throw new Error(`missing plan tool ${tool}: ${names.join(', ')}`);
    }
  }

  const catalogResult = await client.callTool({
    name: 'get_corpus_catalog',
    arguments: { mode: 'latest_per_topic' },
  });
  const catalogText = catalogResult.content?.[0]?.text || '';
  if (!catalogText.includes('demo-topic') || catalogText.includes('common_path')) {
    throw new Error(`unexpected catalog: ${catalogText}`);
  }

  const filesResult = await client.callTool({
    name: 'get_corpus_files',
    arguments: { ids: [DEMO_ID] },
  });
  const filesText = filesResult.content?.[0]?.text || '';
  if (!filesText.includes('knowledge-mcp mock digest')) {
    throw new Error(`unexpected files: ${filesText}`);
  }

  const missingResult = await client.callTool({
    name: 'get_corpus_files',
    arguments: { ids: ['missing-id'] },
  });
  const missingText = missingResult.content?.[0]?.text || '';
  if (!missingText.includes('"ok":false')) {
    throw new Error(`expected per-item error, got: ${missingText}`);
  }

  const archiveDocResult = await client.callTool({
    name: 'archive_document',
    arguments: {
      document:
        '# T\n\n> 创建时间：x\n> 导航：[digest](../../../digest/demo-topic/note.md)\n\n---\n\nbody',
    },
  });
  const archiveDocText = archiveDocResult.content?.[0]?.text || '';
  if (!archiveDocText.includes(DEMO_ID)) {
    throw new Error(`unexpected archive_document: ${archiveDocText}`);
  }

  const archiveDigestResult = await client.callTool({
    name: 'archive_digest',
    arguments: { id: DEMO_ID, digest: '# T — 摘要\n\n## 概述\n\nmock' },
  });
  const archiveDigestText = archiveDigestResult.content?.[0]?.text || '';
  if (!archiveDigestText.includes('digest/demo-topic/note.md')) {
    throw new Error(`unexpected archive_digest: ${archiveDigestText}`);
  }

  planTaskCreateCalls.length = 0;
  resetPlanTaskStore();

  const planOmitResult = await client.callTool({
    name: 'create_plan_task',
    arguments: { title: 'Omit subs' },
  });
  const planOmitText = planOmitResult.content?.[0]?.text || '';
  if (planOmitResult.isError || !planOmitText.includes('task_mock001')) {
    throw new Error(`unexpected create_plan_task omit: ${planOmitText}`);
  }
  if (planTaskCreateCalls.length !== 1 || planTaskCreateCalls[0].title !== 'Omit subs') {
    throw new Error(`unexpected plan-task-create omit payload: ${JSON.stringify(planTaskCreateCalls)}`);
  }

  planTaskCreateCalls.length = 0;
  resetPlanTaskStore();
  const planSingleResult = await client.callTool({
    name: 'create_plan_task',
    arguments: { title: 'One sub', sub_titles: ['Sub A'] },
  });
  const planSingleText = planSingleResult.content?.[0]?.text || '';
  if (planSingleResult.isError || !planSingleText.includes('"implicit":false')) {
    throw new Error(`unexpected create_plan_task single: ${planSingleText}`);
  }
  if (
    planTaskCreateCalls.length !== 1 ||
    JSON.stringify(planTaskCreateCalls[0]) !== JSON.stringify({ title: 'One sub', sub_titles: ['Sub A'] })
  ) {
    throw new Error(`unexpected plan-task-create single payload: ${JSON.stringify(planTaskCreateCalls)}`);
  }

  planTaskCreateCalls.length = 0;
  resetPlanTaskStore();
  const planMultiResult = await client.callTool({
    name: 'create_plan_task',
    arguments: { title: 'Two subs', sub_titles: ['Sub A', 'Sub B'] },
  });
  const planMultiText = planMultiResult.content?.[0]?.text || '';
  if (planMultiResult.isError || !planMultiText.includes('task_mock001_sub_02')) {
    throw new Error(`unexpected create_plan_task multi: ${planMultiText}`);
  }

  const planInvalidResult = await client.callTool({
    name: 'create_plan_task',
    arguments: { title: 'Bad', sub_titles: ['ok', '  '] },
  });
  if (!planInvalidResult.isError) {
    throw new Error('expected create_plan_task validation error for blank sub_titles element');
  }

  resetPlanTaskStore();
  const crudCreate = await client.callTool({
    name: 'create_plan_task',
    arguments: { title: 'CRUD master', sub_titles: ['Sub A', 'Sub B'] },
  });
  const crudCreateText = crudCreate.content?.[0]?.text || '';
  if (crudCreate.isError || !crudCreateText.includes('task_mock001')) {
    throw new Error(`unexpected CRUD create: ${crudCreateText}`);
  }

  const listResult = await client.callTool({ name: 'list_plan_tasks', arguments: {} });
  const listText = listResult.content?.[0]?.text || '';
  if (listResult.isError || !listText.includes('CRUD master')) {
    throw new Error(`unexpected list_plan_tasks: ${listText}`);
  }

  const getResult = await client.callTool({
    name: 'get_plan_task',
    arguments: { id: 'task_mock001' },
  });
  const getText = getResult.content?.[0]?.text || '';
  if (getResult.isError || !getText.includes('Sub B')) {
    throw new Error(`unexpected get_plan_task: ${getText}`);
  }

  const getMissing = await client.callTool({
    name: 'get_plan_task',
    arguments: { id: 'missing-master-id' },
  });
  if (!getMissing.isError) {
    throw new Error('expected get_plan_task error for unknown id');
  }

  const addSub = await client.callTool({
    name: 'add_plan_sub',
    arguments: { master_task_id: 'task_mock001', title: 'Sub C' },
  });
  const addSubText = addSub.content?.[0]?.text || '';
  if (addSub.isError || !addSubText.includes('Sub C')) {
    throw new Error(`unexpected add_plan_sub: ${addSubText}`);
  }

  const parsedAdd = JSON.parse(addSubText);
  const subToComplete = parsedAdd.task.sub_tasks.find((s) => s.title === 'Sub A').sub_task_id;

  const completeSub = await client.callTool({
    name: 'complete_plan_sub',
    arguments: { master_task_id: 'task_mock001', sub_task_id: subToComplete },
  });
  const completeText = completeSub.content?.[0]?.text || '';
  if (completeSub.isError || !completeText.includes('complete')) {
    throw new Error(`unexpected complete_plan_sub: ${completeText}`);
  }

  const linkArchive = await client.callTool({
    name: 'link_plan_archive',
    arguments: {
      master_task_id: 'task_mock001',
      sub_task_id: subToComplete,
      archive_id: DEMO_ID,
    },
  });
  const linkText = linkArchive.content?.[0]?.text || '';
  if (linkArchive.isError || !linkText.includes(DEMO_ID)) {
    throw new Error(`unexpected link_plan_archive: ${linkText}`);
  }

  const subToDelete = parsedAdd.task.sub_tasks.find((s) => s.title === 'Sub B').sub_task_id;
  const deleteSub = await client.callTool({
    name: 'delete_plan_sub',
    arguments: { master_task_id: 'task_mock001', sub_task_id: subToDelete },
  });
  if (deleteSub.isError) {
    throw new Error(`unexpected delete_plan_sub: ${deleteSub.content?.[0]?.text || ''}`);
  }

  const deleteMaster = await client.callTool({
    name: 'delete_plan_task',
    arguments: { master_task_id: 'task_mock001' },
  });
  const deleteMasterText = deleteMaster.content?.[0]?.text || '';
  if (deleteMaster.isError || !deleteMasterText.includes('"ok":true')) {
    throw new Error(`unexpected delete_plan_task: ${deleteMasterText}`);
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
