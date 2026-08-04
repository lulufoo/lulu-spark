#!/usr/bin/env node
/**
 * knowledge-mcp verification: mock Workbench HTTP + sidecar MCP client (TDD / CI).
 */
import { spawn } from 'node:child_process';
import fs from 'node:fs';
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

/** @type {Array<{ title: string, todo_md?: string }>} */
const planTaskCreateCalls = [];

/** Breaking rename: old MCP tool name must not remain registered (T8 / AC5 / R1). */
const FORBIDDEN_PLAN_TOOL_NAMES = [
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

/** Master status wire values — list/get/create readback must admit all three (T8 / AC7). */
const MASTER_STATUS_WIRE = ['incomplete', 'complete', 'abandoned'];

/** T10 / AC-等价 — full todo tool set (complete/link/attachment/update required; no complete_plan_sub). */
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

/** t5 / AC3 — additive sub-content surface (not part of T10 13-tool EQUIVALENCE). */
const SUB_CONTENT_TODO_TOOLS = ['update_todo_sub'];

const ATTACHMENT_TOOL_HTTP_PATHS = {
  add_todo_attachment: '/api/todo-task-add-attachment',
  list_todo_attachments: '/api/todo-task-list-attachments',
  get_todo_attachment: '/api/todo-task-get-attachment',
  update_todo_attachment: '/api/todo-task-update-attachment',
};

const FORBIDDEN_ATTACHMENT_DELETE_TOOL_NAMES = [
  'delete_plan_attachment',
  'remove_plan_attachment',
  'delete_todo_attachment',
  'remove_todo_attachment',
];

function assertMasterStatusWire(status, label) {
  if (typeof status !== 'string' || !MASTER_STATUS_WIRE.includes(status)) {
    throw new Error(
      `${label}: status must be one of ${MASTER_STATUS_WIRE.join('|')}, got ${JSON.stringify(status)}`,
    );
  }
}

function seedMaster(masterId, title, status) {
  planTaskStore.set(masterId, {
    master_task_id: masterId,
    title,
    status,
    created_at: '2026-07-07T00:00:00Z',
    sub_tasks: [],
    todo_md: '',
    migration_error: false,
  });
}

/** @type {Map<string, object>} */
const planTaskStore = new Map();
let planSubSeq = 0;

/** @type {Map<string, Array<{ file_name: string, original_file_name: string, content: string, added_at: string }>>} */
const planAttachmentStore = new Map();

/** @type {Array<{ path: string, body: object }>} */
const planAttachmentHttpCalls = [];

function resetPlanTaskStore() {
  planTaskStore.clear();
  planSubSeq = 0;
  planAttachmentStore.clear();
  planAttachmentHttpCalls.length = 0;
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

    if (req.method === 'GET' && url.pathname === '/api/todo-tasks') {
      respondJson(res, 200, planTaskSnapshot());
      return;
    }

    if (req.method === 'GET' && url.pathname === '/api/todo-task') {
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

    if (req.method === 'POST' && url.pathname === '/api/todo-task-create') {
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
      if (typeof payload.todo_md !== 'undefined' && typeof payload.todo_md !== 'string') {
        respondJson(res, 400, { error: 'Invalid todo_md' });
        return;
      }
      const planMd = typeof payload.todo_md === 'string' ? payload.todo_md : '';
      planTaskCreateCalls.push({
        title,
        ...(planMd !== '' ? { todo_md: planMd } : {}),
      });
      const masterId = 'task_mock001';
      const task = {
        master_task_id: masterId,
        title,
        status: 'incomplete',
        created_at: '2026-07-07T00:00:00Z',
        sub_tasks: [],
        todo_md: planMd,
        migration_error: false,
      };
      planTaskStore.set(masterId, task);
      respondJson(res, 201, {
        master_task_id: masterId,
        task,
      });
      return;
    }

    if (req.method === 'POST' && url.pathname === '/api/todo-task-update') {
      let payload;
      try {
        payload = await readJsonBody(req);
      } catch {
        respondJson(res, 400, { error: 'Invalid JSON' });
        return;
      }
      const masterId =
        typeof payload.master_task_id === 'string' ? payload.master_task_id.trim() : '';
      if (!masterId) {
        respondJson(res, 400, { error: 'Missing master_task_id' });
        return;
      }
      const hasTitle = Object.prototype.hasOwnProperty.call(payload, 'title');
      const hasTodoMd = Object.prototype.hasOwnProperty.call(payload, 'todo_md');
      if (!hasTitle && !hasTodoMd) {
        respondJson(res, 400, { error: 'Missing title or todo_md' });
        return;
      }
      if (hasTitle && typeof payload.title !== 'string') {
        respondJson(res, 400, { error: 'Invalid title' });
        return;
      }
      if (hasTodoMd && typeof payload.todo_md !== 'string') {
        respondJson(res, 400, { error: 'Invalid todo_md' });
        return;
      }
      const existing = planTaskStore.get(masterId);
      if (!existing) {
        respondJson(res, 404, { error: 'Task not found' });
        return;
      }
      if (hasTitle) {
        const title = payload.title.trim();
        if (!title) {
          respondJson(res, 400, { error: 'Missing title' });
          return;
        }
        existing.title = title;
      }
      if (hasTodoMd) {
        existing.todo_md = payload.todo_md;
      }
      planTaskStore.set(masterId, existing);
      respondJson(res, 200, { task: { ...existing } });
      return;
    }

    if (req.method === 'POST' && url.pathname === '/api/todo-task-delete') {
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

    if (req.method === 'POST' && url.pathname === '/api/todo-task-add-sub') {
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
      if (typeof payload.content === 'string' && payload.content !== '') {
        sub.content = payload.content;
      }
      task.sub_tasks.push(sub);
      respondJson(res, 201, { sub_task_id: sub.sub_task_id, task });
      return;
    }

    if (req.method === 'POST' && url.pathname === '/api/todo-task-update-sub') {
      let payload;
      try {
        payload = await readJsonBody(req);
      } catch {
        respondJson(res, 400, { error: 'Invalid JSON' });
        return;
      }
      const masterId = typeof payload.master_task_id === 'string' ? payload.master_task_id.trim() : '';
      const subId = typeof payload.sub_task_id === 'string' ? payload.sub_task_id.trim() : '';
      const title = typeof payload.title === 'string' ? payload.title.trim() : '';
      if (!masterId) {
        respondJson(res, 400, { error: 'Missing master_task_id' });
        return;
      }
      if (!subId) {
        respondJson(res, 400, { error: 'Missing sub_task_id' });
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
      const sub = task.sub_tasks.find((s) => s.sub_task_id === subId);
      if (!sub) {
        respondJson(res, 404, { error: 'Sub task not found' });
        return;
      }
      sub.title = title;
      if (Object.prototype.hasOwnProperty.call(payload, 'content')) {
        const content = typeof payload.content === 'string' ? payload.content : '';
        if (content === '') {
          delete sub.content;
        } else {
          sub.content = content;
        }
      }
      respondJson(res, 200, { task });
      return;
    }

    if (req.method === 'POST' && url.pathname === '/api/todo-task-delete-sub') {
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

    if (req.method === 'POST' && url.pathname === '/api/todo-task-complete') {
      let payload;
      try {
        payload = await readJsonBody(req);
      } catch {
        respondJson(res, 400, { error: 'Invalid JSON' });
        return;
      }
      const masterId = typeof payload.master_task_id === 'string' ? payload.master_task_id.trim() : '';
      const subId =
        typeof payload.sub_task_id === 'string' ? payload.sub_task_id.trim() : '';
      if (!masterId) {
        respondJson(res, 400, { error: 'Missing master_task_id' });
        return;
      }
      const task = planTaskStore.get(masterId);
      if (!task) {
        respondJson(res, 404, { error: 'Task not found' });
        return;
      }
      // Mirror host complete_todo: abandoned rejects; already-complete is idempotent (T5/T8).
      if (task.status === 'abandoned') {
        respondJson(res, 409, { error: 'master_abandoned' });
        return;
      }
      if (!subId) {
        task.status = 'complete';
        respondJson(res, 200, { task });
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

    if (req.method === 'POST' && url.pathname === '/api/todo-task-link-archive') {
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

    if (req.method === 'POST' && url.pathname === '/api/todo-task-add-attachment') {
      let payload;
      try {
        payload = await readJsonBody(req);
      } catch {
        respondJson(res, 400, { error: 'Invalid JSON' });
        return;
      }
      planAttachmentHttpCalls.push({ path: url.pathname, body: payload });
      const masterId = typeof payload.master_task_id === 'string' ? payload.master_task_id.trim() : '';
      const fileName = typeof payload.file_name === 'string' ? payload.file_name.trim() : '';
      const content = typeof payload.content === 'string' ? payload.content : null;
      if (!masterId) {
        respondJson(res, 400, { error: 'Missing master_task_id' });
        return;
      }
      if (!fileName) {
        respondJson(res, 400, { error: 'Missing file_name' });
        return;
      }
      if (content === null) {
        respondJson(res, 400, { error: 'Missing content' });
        return;
      }
      if (!planTaskStore.has(masterId)) {
        respondJson(res, 404, { error: 'Task not found' });
        return;
      }
      const entry = {
        file_name: fileName,
        original_file_name: fileName,
        content,
        added_at: '2026-07-18T00:00:00Z',
      };
      const list = planAttachmentStore.get(masterId) || [];
      list.push(entry);
      planAttachmentStore.set(masterId, list);
      respondJson(res, 201, {
        file_name: entry.file_name,
        original_file_name: entry.original_file_name,
        added_at: entry.added_at,
      });
      return;
    }

    if (req.method === 'POST' && url.pathname === '/api/todo-task-list-attachments') {
      let payload;
      try {
        payload = await readJsonBody(req);
      } catch {
        respondJson(res, 400, { error: 'Invalid JSON' });
        return;
      }
      planAttachmentHttpCalls.push({ path: url.pathname, body: payload });
      const masterId = typeof payload.master_task_id === 'string' ? payload.master_task_id.trim() : '';
      if (!masterId) {
        respondJson(res, 400, { error: 'Missing master_task_id' });
        return;
      }
      if (!planTaskStore.has(masterId)) {
        respondJson(res, 404, { error: 'Task not found' });
        return;
      }
      const attachments = (planAttachmentStore.get(masterId) || []).map(
        ({ file_name, original_file_name, added_at }) => ({
          file_name,
          original_file_name,
          added_at,
        }),
      );
      respondJson(res, 200, { attachments });
      return;
    }

    if (req.method === 'POST' && url.pathname === '/api/todo-task-get-attachment') {
      let payload;
      try {
        payload = await readJsonBody(req);
      } catch {
        respondJson(res, 400, { error: 'Invalid JSON' });
        return;
      }
      planAttachmentHttpCalls.push({ path: url.pathname, body: payload });
      const masterId = typeof payload.master_task_id === 'string' ? payload.master_task_id.trim() : '';
      const fileName = typeof payload.file_name === 'string' ? payload.file_name.trim() : '';
      if (!masterId || !fileName) {
        respondJson(res, 400, { error: 'Missing master_task_id or file_name' });
        return;
      }
      if (!planTaskStore.has(masterId)) {
        respondJson(res, 404, { error: 'Task not found' });
        return;
      }
      const entry = (planAttachmentStore.get(masterId) || []).find((a) => a.file_name === fileName);
      if (!entry) {
        respondJson(res, 404, { error: 'Attachment not found' });
        return;
      }
      respondJson(res, 200, { file_name: entry.file_name, content: entry.content });
      return;
    }

    if (req.method === 'POST' && url.pathname === '/api/todo-task-update-attachment') {
      let payload;
      try {
        payload = await readJsonBody(req);
      } catch {
        respondJson(res, 400, { error: 'Invalid JSON' });
        return;
      }
      planAttachmentHttpCalls.push({ path: url.pathname, body: payload });
      const masterId = typeof payload.master_task_id === 'string' ? payload.master_task_id.trim() : '';
      const fileName = typeof payload.file_name === 'string' ? payload.file_name.trim() : '';
      const content = typeof payload.content === 'string' ? payload.content : null;
      if (!masterId || !fileName) {
        respondJson(res, 400, { error: 'Missing master_task_id or file_name' });
        return;
      }
      if (content === null) {
        respondJson(res, 400, { error: 'Missing content' });
        return;
      }
      if (!planTaskStore.has(masterId)) {
        respondJson(res, 404, { error: 'Task not found' });
        return;
      }
      const list = planAttachmentStore.get(masterId) || [];
      const entry = list.find((a) => a.file_name === fileName);
      if (!entry) {
        respondJson(res, 404, { error: 'Attachment not found' });
        return;
      }
      entry.content = content;
      respondJson(res, 200, { ok: true });
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

/**
 * Connect to path scene_slot MCP URL and list tool names.
 * @param {number} mcpPort
 * @param {string} sceneSlot — path segment after /mcp/ (e.g. todo_task)
 */
async function connectAndListToolNames(mcpPort, sceneSlot) {
  const mcpUrl = `http://127.0.0.1:${mcpPort}/mcp/${encodeURIComponent(sceneSlot)}`;
  const transport = new StreamableHTTPClientTransport(new URL(mcpUrl));
  const client = new Client({ name: 'knowledge-mcp-verify-slot', version: '0.1.0' });
  await client.connect(transport);
  const tools = await client.listTools();
  const names = tools.tools.map((t) => t.name).sort();
  await client.close();
  return names;
}

/** Attempt connect+listTools; returns { ok, names?, error? }. Never throws. */
async function tryConnectAndListToolNames(mcpPort, pathSuffix) {
  const mcpUrl = `http://127.0.0.1:${mcpPort}${pathSuffix}`;
  try {
    const transport = new StreamableHTTPClientTransport(new URL(mcpUrl));
    const client = new Client({ name: 'knowledge-mcp-verify-probe', version: '0.1.0' });
    await client.connect(transport);
    const tools = await client.listTools();
    const names = tools.tools.map((t) => t.name).sort();
    await client.close();
    return { ok: true, names };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
}

function assertIncludesAll(names, required, label) {
  for (const tool of required) {
    if (!names.includes(tool)) {
      throw new Error(`${label}: missing ${tool}; got ${names.join(', ')}`);
    }
  }
}

function assertNoneOf(names, forbidden, label) {
  for (const tool of forbidden) {
    if (names.includes(tool)) {
      throw new Error(`${label}: unexpected ${tool}; got ${names.join(', ')}`);
    }
  }
}

const CORPUS_TOOLS = [
  'get_corpus_catalog',
  'get_corpus_files',
  'archive_document',
  'archive_digest',
];
const TODO_SURFACE = [...EQUIVALENCE_TODO_TOOLS, ...SUB_CONTENT_TODO_TOOLS];

/**
 * t1 / AC2–AC4 — path scene_slot routing:
 * /mcp/todo_task and /mcp/cursor_ide expose distinguishable API sets;
 * unknown path hard-fails (no full-tool fallback); old mount silent fallback gone.
 */
async function testSceneSlotPathRouting(workbenchUrl) {
  const mcpPort = await ephemeralPort();
  const sidecar = spawnSidecar(mcpPort, workbenchUrl);
  const ready = await waitFor(`http://127.0.0.1:${mcpPort}/health`);
  if (!ready) {
    sidecar.kill('SIGTERM');
    throw new Error('scene_slot path: sidecar health timeout');
  }

  try {
    // --- /mcp/todo_task: todo-only surface (aligns with Binding key todo_task) ---
    const todoNames = await connectAndListToolNames(mcpPort, 'todo_task');
    assertIncludesAll(todoNames, TODO_SURFACE, '/mcp/todo_task');
    assertNoneOf(todoNames, CORPUS_TOOLS, '/mcp/todo_task');

    // Stability: repeated tools/list must not jitter (AC2/AC3 isolation observability)
    const todoNamesAgain = await connectAndListToolNames(mcpPort, 'todo_task');
    if (JSON.stringify(todoNames) !== JSON.stringify(todoNamesAgain)) {
      throw new Error(
        `/mcp/todo_task tools/list not stable: ${todoNames.join(',')} vs ${todoNamesAgain.join(',')}`,
      );
    }

    // --- /mcp/cursor_ide: distinguishable from todo_task (corpus/archive seed) ---
    const ideNames = await connectAndListToolNames(mcpPort, 'cursor_ide');
    assertIncludesAll(ideNames, CORPUS_TOOLS, '/mcp/cursor_ide');
    assertNoneOf(ideNames, TODO_SURFACE, '/mcp/cursor_ide');
    if (JSON.stringify(todoNames) === JSON.stringify(ideNames)) {
      throw new Error('todo_task and cursor_ide slots must expose distinguishable tools/list');
    }
    const ideNamesAgain = await connectAndListToolNames(mcpPort, 'cursor_ide');
    if (JSON.stringify(ideNames) !== JSON.stringify(ideNamesAgain)) {
      throw new Error(
        `/mcp/cursor_ide tools/list not stable: ${ideNames.join(',')} vs ${ideNamesAgain.join(',')}`,
      );
    }

    // --- registered slot HTTP success path (todo_task → list_todo_tasks) ---
    {
      const transport = new StreamableHTTPClientTransport(
        new URL(`http://127.0.0.1:${mcpPort}/mcp/todo_task`),
      );
      const client = new Client({ name: 'knowledge-mcp-verify-slot-http', version: '0.1.0' });
      await client.connect(transport);
      const result = await client.callTool({ name: 'list_todo_tasks', arguments: {} });
      if (result.isError) {
        throw new Error(
          `todo_task list_todo_tasks must succeed via Workbench HTTP: ${result.content?.[0]?.text || ''}`,
        );
      }
      await client.close();
    }

    // --- unknown path slot: hard fail; must NOT return full tool set ---
    const unknown = await tryConnectAndListToolNames(mcpPort, '/mcp/__unknown__');
    if (unknown.ok) {
      const fullSurface = [...TODO_SURFACE, ...CORPUS_TOOLS];
      const hasFull =
        fullSurface.every((t) => unknown.names.includes(t)) ||
        (TODO_SURFACE.every((t) => unknown.names.includes(t)) &&
          CORPUS_TOOLS.every((t) => unknown.names.includes(t)));
      if (hasFull || unknown.names.length > 0) {
        // Any successful tools/list on unknown slot is a hard-reject violation if it
        // exposes tools; empty list is also wrong — connection itself must fail.
        throw new Error(
          `unknown scene_slot /mcp/__unknown__ must hard-fail connect/tools/list; got tools: ${unknown.names.join(', ')}`,
        );
      }
      throw new Error('unknown scene_slot /mcp/__unknown__ must hard-fail connect/tools/list');
    }

    // --- old bare /mcp and ?mount= / MCP_MOUNT silent full-tool fallback unreachable ---
    const bare = await tryConnectAndListToolNames(mcpPort, '/mcp');
    if (bare.ok) {
      const exposesFull =
        TODO_SURFACE.every((t) => bare.names.includes(t)) &&
        CORPUS_TOOLS.every((t) => bare.names.includes(t));
      if (exposesFull) {
        throw new Error('bare /mcp must not expose full tool set (old default mount fallback)');
      }
      // Bare /mcp without scene_slot is not a registered path slot — must hard-fail.
      throw new Error(
        `bare /mcp must hard-fail (path scene_slot is sole isolation surface); got tools: ${bare.names.join(', ')}`,
      );
    }

    const mountQuery = await tryConnectAndListToolNames(mcpPort, '/mcp?mount=todo');
    if (mountQuery.ok) {
      throw new Error(
        `?mount= query must not select tools (old mount path); got: ${mountQuery.names.join(', ')}`,
      );
    }

    // Static: silent-fallback mount helpers must be removed from index.mjs
    const indexSrc = fs.readFileSync(path.join(PKG_ROOT, 'index.mjs'), 'utf8');
    for (const forbidden of [
      'KNOWN_MOUNTS',
      'resolveMountKey',
      'mountKeyFromRequest',
      'MCP_MOUNT',
    ]) {
      if (indexSrc.includes(forbidden)) {
        throw new Error(
          `index.mjs must remove old mount silent-fallback surface; still contains ${forbidden}`,
        );
      }
    }
  } finally {
    sidecar.kill('SIGTERM');
    await sleep(200);
  }

  // MCP_MOUNT env must not resurrect full-tool silent fallback on any reachable path
  {
    const envPort = await ephemeralPort();
    const envSidecar = spawnSidecar(envPort, workbenchUrl, { MCP_MOUNT: 'not-a-real-mount' });
    const envReady = await waitFor(`http://127.0.0.1:${envPort}/health`);
    if (!envReady) {
      envSidecar.kill('SIGTERM');
      throw new Error('MCP_MOUNT env probe: sidecar health timeout');
    }
    try {
      const viaBare = await tryConnectAndListToolNames(envPort, '/mcp');
      if (viaBare.ok) {
        const exposesFull =
          TODO_SURFACE.every((t) => viaBare.names.includes(t)) &&
          CORPUS_TOOLS.every((t) => viaBare.names.includes(t));
        if (exposesFull) {
          throw new Error('MCP_MOUNT unknown must not silently fall back to full tools on /mcp');
        }
        throw new Error('MCP_MOUNT must not open bare /mcp tool surface');
      }
      // Registered path slots must still work and ignore MCP_MOUNT
      const todoNames = await connectAndListToolNames(envPort, 'todo_task');
      assertIncludesAll(todoNames, TODO_SURFACE, 'todo_task ignores MCP_MOUNT');
      assertNoneOf(todoNames, CORPUS_TOOLS, 'todo_task ignores MCP_MOUNT');
    } finally {
      envSidecar.kill('SIGTERM');
      await sleep(200);
    }
  }
}

/**
 * t1 — when Workbench HTTP is unreachable, tools/call returns isError (sidecar stays up).
 */
async function testUnreachableWorkbenchHttp() {
  const mcpPort = await ephemeralPort();
  const deadHttpPort = await ephemeralPort();
  const workbenchUrl = `http://127.0.0.1:${deadHttpPort}`;
  const sidecar = spawnSidecar(mcpPort, workbenchUrl);
  const ready = await waitFor(`http://127.0.0.1:${mcpPort}/health`);
  if (!ready) {
    sidecar.kill('SIGTERM');
    throw new Error('unreachable-http: sidecar health timeout');
  }

  const transport = new StreamableHTTPClientTransport(
    new URL(`http://127.0.0.1:${mcpPort}/mcp/todo_task`),
  );
  const client = new Client({ name: 'knowledge-mcp-verify-down', version: '0.1.0' });
  await client.connect(transport);
  const result = await client.callTool({ name: 'list_todo_tasks', arguments: {} });
  if (!result.isError) {
    await client.close();
    sidecar.kill('SIGTERM');
    throw new Error('expected list_todo_tasks isError when Workbench HTTP unreachable');
  }
  const health = await fetch(`http://127.0.0.1:${mcpPort}/health`);
  if (!health.ok) {
    await client.close();
    sidecar.kill('SIGTERM');
    throw new Error('sidecar crashed after tools/call with unreachable HTTP');
  }
  await client.close();
  sidecar.kill('SIGTERM');
  await sleep(200);
}

async function runMcpClient(mcpPort) {
  // cursor_ide slot: corpus/archive tools only
  const ideUrl = `http://127.0.0.1:${mcpPort}/mcp/cursor_ide`;
  const ideTransport = new StreamableHTTPClientTransport(new URL(ideUrl));
  const ideClient = new Client({ name: 'knowledge-mcp-verify-ide', version: '0.1.0' });
  await ideClient.connect(ideTransport);

  const ideTools = await ideClient.listTools();
  const ideNames = ideTools.tools.map((t) => t.name);
  if (!ideNames.includes('get_corpus_catalog') || !ideNames.includes('get_corpus_files')) {
    throw new Error(`cursor_ide missing corpus tools: ${ideNames.join(', ')}`);
  }
  if (!ideNames.includes('archive_document') || !ideNames.includes('archive_digest')) {
    throw new Error(`cursor_ide missing archive tools: ${ideNames.join(', ')}`);
  }
  for (const tool of TODO_SURFACE) {
    if (ideNames.includes(tool)) {
      throw new Error(`cursor_ide must not expose todo tool ${tool}`);
    }
  }

  const catalogResult = await ideClient.callTool({
    name: 'get_corpus_catalog',
    arguments: { mode: 'latest_per_topic' },
  });
  const catalogText = catalogResult.content?.[0]?.text || '';
  if (!catalogText.includes('demo-topic') || catalogText.includes('common_path')) {
    throw new Error(`unexpected catalog: ${catalogText}`);
  }

  const filesResult = await ideClient.callTool({
    name: 'get_corpus_files',
    arguments: { ids: [DEMO_ID] },
  });
  const filesText = filesResult.content?.[0]?.text || '';
  if (!filesText.includes('knowledge-mcp mock digest')) {
    throw new Error(`unexpected files: ${filesText}`);
  }

  const missingResult = await ideClient.callTool({
    name: 'get_corpus_files',
    arguments: { ids: ['missing-id'] },
  });
  const missingText = missingResult.content?.[0]?.text || '';
  if (!missingText.includes('"ok":false')) {
    throw new Error(`expected per-item error, got: ${missingText}`);
  }

  const archiveDocResult = await ideClient.callTool({
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

  const archiveDigestResult = await ideClient.callTool({
    name: 'archive_digest',
    arguments: { id: DEMO_ID, digest: '# T — 摘要\n\n## 概述\n\nmock' },
  });
  const archiveDigestText = archiveDigestResult.content?.[0]?.text || '';
  if (!archiveDigestText.includes('digest/demo-topic/note.md')) {
    throw new Error(`unexpected archive_digest: ${archiveDigestText}`);
  }
  await ideClient.close();

  // todo_task slot: todo tools only
  const mcpUrl = `http://127.0.0.1:${mcpPort}/mcp/todo_task`;
  const transport = new StreamableHTTPClientTransport(new URL(mcpUrl));
  const client = new Client({ name: 'knowledge-mcp-verify', version: '0.1.0' });
  await client.connect(transport);

  const tools = await client.listTools();
  const names = tools.tools.map((t) => t.name);
  if (!names.includes('create_todo_task')) {
    throw new Error(`missing create_todo_task tool: ${names.join(', ')}`);
  }
  for (const tool of EQUIVALENCE_TODO_TOOLS) {
    if (!names.includes(tool)) {
      throw new Error(`missing equivalence todo tool ${tool}: ${names.join(', ')}`);
    }
  }
  for (const tool of SUB_CONTENT_TODO_TOOLS) {
    if (!names.includes(tool)) {
      throw new Error(`missing sub-content todo tool ${tool}: ${names.join(', ')}`);
    }
  }
  for (const tool of CORPUS_TOOLS) {
    if (names.includes(tool)) {
      throw new Error(`todo_task must not expose corpus tool ${tool}`);
    }
  }
  for (const tool of FORBIDDEN_PLAN_TOOL_NAMES) {
    if (names.includes(tool)) {
      throw new Error(`forbidden plan_* tool still registered: ${tool}`);
    }
  }
  for (const tool of FORBIDDEN_ATTACHMENT_DELETE_TOOL_NAMES) {
    if (names.includes(tool)) {
      throw new Error(`forbidden attachment delete tool registered: ${tool}`);
    }
  }

  planTaskCreateCalls.length = 0;
  resetPlanTaskStore();

  const planOmitResult = await client.callTool({
    name: 'create_todo_task',
    arguments: { title: 'Omit body' },
  });
  const planOmitText = planOmitResult.content?.[0]?.text || '';
  if (planOmitResult.isError || !planOmitText.includes('task_mock001')) {
    throw new Error(`unexpected create_todo_task omit: ${planOmitText}`);
  }
  if (
    planTaskCreateCalls.length !== 1 ||
    JSON.stringify(planTaskCreateCalls[0]) !== JSON.stringify({ title: 'Omit body' })
  ) {
    throw new Error(`unexpected plan-task-create omit payload: ${JSON.stringify(planTaskCreateCalls)}`);
  }
  const omitTask = JSON.parse(planOmitText).task;
  if (!Array.isArray(omitTask.sub_tasks) || omitTask.sub_tasks.length !== 0) {
    throw new Error(`expected empty sub_tasks on create, got: ${planOmitText}`);
  }
  if (omitTask.status !== 'incomplete') {
    throw new Error(`create_todo_task status must be incomplete, got: ${planOmitText}`);
  }
  assertMasterStatusWire(omitTask.status, 'create_todo_task');

  planTaskCreateCalls.length = 0;
  resetPlanTaskStore();
  const planBodyResult = await client.callTool({
    name: 'create_todo_task',
    arguments: { title: 'With body', todo_md: '## Notes\n\nHello' },
  });
  const planBodyText = planBodyResult.content?.[0]?.text || '';
  if (planBodyResult.isError || !planBodyText.includes('task_mock001')) {
    throw new Error(`unexpected create_todo_task with todo_md: ${planBodyText}`);
  }
  if (
    planTaskCreateCalls.length !== 1 ||
    JSON.stringify(planTaskCreateCalls[0]) !==
      JSON.stringify({ title: 'With body', todo_md: '## Notes\n\nHello' })
  ) {
    throw new Error(`unexpected plan-task-create body payload: ${JSON.stringify(planTaskCreateCalls)}`);
  }
  if (!planBodyText.includes('## Notes')) {
    throw new Error(`create response missing todo_md echo: ${planBodyText}`);
  }

  const planTitleLong = await client.callTool({
    name: 'create_todo_task',
    arguments: {
      title:
        'one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twentyone',
    },
  });
  if (!planTitleLong.isError) {
    throw new Error('expected create_todo_task validation error for title over 20 words');
  }

  resetPlanTaskStore();
  const crudCreate = await client.callTool({
    name: 'create_todo_task',
    arguments: { title: 'CRUD master', todo_md: '# Plan' },
  });
  const crudCreateText = crudCreate.content?.[0]?.text || '';
  if (crudCreate.isError || !crudCreateText.includes('task_mock001')) {
    throw new Error(`unexpected CRUD create: ${crudCreateText}`);
  }

  const addSubA = await client.callTool({
    name: 'add_todo_sub',
    arguments: { master_task_id: 'task_mock001', title: 'Sub A' },
  });
  if (addSubA.isError) {
    throw new Error(`unexpected add_todo_sub A: ${addSubA.content?.[0]?.text || ''}`);
  }
  const addSubB = await client.callTool({
    name: 'add_todo_sub',
    arguments: { master_task_id: 'task_mock001', title: 'Sub B' },
  });
  if (addSubB.isError) {
    throw new Error(`unexpected add_todo_sub B: ${addSubB.content?.[0]?.text || ''}`);
  }

  const listResult = await client.callTool({ name: 'list_todo_tasks', arguments: {} });
  const listText = listResult.content?.[0]?.text || '';
  if (listResult.isError || !listText.includes('CRUD master')) {
    throw new Error(`unexpected list_todo_tasks before update: ${listText}`);
  }
  const listParsed = JSON.parse(listText);
  const listMaster = listParsed.find((t) => t.master_task_id === 'task_mock001');
  if (!listMaster) {
    throw new Error(`list_todo_tasks missing CRUD master: ${listText}`);
  }
  assertMasterStatusWire(listMaster.status, 'list_todo_tasks');

  const getResult = await client.callTool({
    name: 'get_todo_task',
    arguments: { id: 'task_mock001' },
  });
  const getText = getResult.content?.[0]?.text || '';
  if (getResult.isError || !getText.includes('Sub B')) {
    throw new Error(`unexpected get_todo_task: ${getText}`);
  }
  assertMasterStatusWire(JSON.parse(getText).status, 'get_todo_task');

  const updateTitleOnly = await client.callTool({
    name: 'update_todo_task',
    arguments: { master_task_id: 'task_mock001', title: 'CRUD renamed' },
  });
  const updateTitleText = updateTitleOnly.content?.[0]?.text || '';
  if (updateTitleOnly.isError || !updateTitleText.includes('CRUD renamed')) {
    throw new Error(`unexpected update_todo_task title: ${updateTitleText}`);
  }
  const updateTitleTask = JSON.parse(updateTitleText).task;
  if (updateTitleTask.todo_md !== '# Plan') {
    throw new Error(`update_todo_task title-only must preserve todo_md: ${updateTitleText}`);
  }

  const updateBodyOnly = await client.callTool({
    name: 'update_todo_task',
    arguments: { master_task_id: 'task_mock001', todo_md: '# Updated plan' },
  });
  const updateBodyText = updateBodyOnly.content?.[0]?.text || '';
  if (updateBodyOnly.isError || !updateBodyText.includes('# Updated plan')) {
    throw new Error(`unexpected update_todo_task body: ${updateBodyText}`);
  }
  const updateBodyTask = JSON.parse(updateBodyText).task;
  if (updateBodyTask.title !== 'CRUD renamed') {
    throw new Error(`update_todo_task body-only must preserve title: ${updateBodyText}`);
  }

  const updateNeither = await client.callTool({
    name: 'update_todo_task',
    arguments: { master_task_id: 'task_mock001' },
  });
  if (!updateNeither.isError) {
    throw new Error('expected update_todo_task error when neither title nor todo_md provided');
  }

  const getMissing = await client.callTool({
    name: 'get_todo_task',
    arguments: { id: 'missing-master-id' },
  });
  if (!getMissing.isError) {
    throw new Error('expected get_todo_task error for unknown id');
  }

  const addSub = await client.callTool({
    name: 'add_todo_sub',
    arguments: { master_task_id: 'task_mock001', title: 'Sub C' },
  });
  const addSubText = addSub.content?.[0]?.text || '';
  if (addSub.isError || !addSubText.includes('Sub C')) {
    throw new Error(`unexpected add_todo_sub: ${addSubText}`);
  }

  // t5 / AC2 — create-with-content round-trip via get
  const addWithContent = await client.callTool({
    name: 'add_todo_sub',
    arguments: {
      master_task_id: 'task_mock001',
      title: 'Sub with content',
      content: 'verify content body',
    },
  });
  const addWithContentText = addWithContent.content?.[0]?.text || '';
  if (addWithContent.isError || !addWithContentText.includes('verify content body')) {
    throw new Error(`unexpected add_todo_sub with content: ${addWithContentText}`);
  }
  const contentSubId = JSON.parse(addWithContentText).task.sub_tasks.find(
    (s) => s.title === 'Sub with content',
  )?.sub_task_id;
  if (!contentSubId) {
    throw new Error(`add_todo_sub with content missing sub id: ${addWithContentText}`);
  }

  // t5 / AC3 — update_todo_sub modify / omit / clear (content: '')
  const updateSubSet = await client.callTool({
    name: 'update_todo_sub',
    arguments: {
      master_task_id: 'task_mock001',
      sub_task_id: contentSubId,
      content: 'verify content v2',
    },
  });
  const updateSubSetText = updateSubSet.content?.[0]?.text || '';
  if (updateSubSet.isError || !updateSubSetText.includes('verify content v2')) {
    throw new Error(`unexpected update_todo_sub set content: ${updateSubSetText}`);
  }

  const updateSubOmit = await client.callTool({
    name: 'update_todo_sub',
    arguments: {
      master_task_id: 'task_mock001',
      sub_task_id: contentSubId,
      title: 'Sub with content renamed',
    },
  });
  const updateSubOmitText = updateSubOmit.content?.[0]?.text || '';
  if (updateSubOmit.isError) {
    throw new Error(`unexpected update_todo_sub omit content: ${updateSubOmitText}`);
  }
  const updateSubOmitTask = JSON.parse(updateSubOmitText).task;
  const omitSub = updateSubOmitTask.sub_tasks.find((s) => s.sub_task_id === contentSubId);
  if (!omitSub || omitSub.content !== 'verify content v2') {
    throw new Error(`update_todo_sub omit content must leave content unchanged: ${updateSubOmitText}`);
  }
  if (omitSub.title !== 'Sub with content renamed') {
    throw new Error(`update_todo_sub title rename failed: ${updateSubOmitText}`);
  }

  const updateSubClear = await client.callTool({
    name: 'update_todo_sub',
    arguments: {
      master_task_id: 'task_mock001',
      sub_task_id: contentSubId,
      content: '',
    },
  });
  const updateSubClearText = updateSubClear.content?.[0]?.text || '';
  if (updateSubClear.isError) {
    throw new Error(`unexpected update_todo_sub clear content: ${updateSubClearText}`);
  }
  const clearSub = JSON.parse(updateSubClearText).task.sub_tasks.find(
    (s) => s.sub_task_id === contentSubId,
  );
  if (!clearSub || (clearSub.content != null && clearSub.content !== '')) {
    throw new Error(`update_todo_sub content: '' must clear content: ${updateSubClearText}`);
  }

  const parsedAdd = JSON.parse(addSubText);
  const subToComplete = parsedAdd.task.sub_tasks.find((s) => s.title === 'Sub A').sub_task_id;

  const completeSub = await client.callTool({
    name: 'complete_todo',
    arguments: { master_task_id: 'task_mock001', sub_task_id: subToComplete },
  });
  const completeText = completeSub.content?.[0]?.text || '';
  if (completeSub.isError || !completeText.includes('complete')) {
    throw new Error(`unexpected complete_todo with sub_task_id: ${completeText}`);
  }
  const afterSubComplete = JSON.parse(completeText).task;
  const completedSub = afterSubComplete.sub_tasks.find((s) => s.sub_task_id === subToComplete);
  if (!completedSub || completedSub.status !== 'complete') {
    throw new Error(`complete_todo with sub_task_id must mark sub complete: ${completeText}`);
  }
  if (afterSubComplete.status !== 'incomplete') {
    throw new Error(
      `complete_todo with sub_task_id must not rewrite master status, got: ${completeText}`,
    );
  }

  const linkArchive = await client.callTool({
    name: 'link_todo_archive',
    arguments: {
      master_task_id: 'task_mock001',
      sub_task_id: subToComplete,
      archive_id: DEMO_ID,
    },
  });
  const linkText = linkArchive.content?.[0]?.text || '';
  if (linkArchive.isError || !linkText.includes(DEMO_ID)) {
    throw new Error(`unexpected link_todo_archive: ${linkText}`);
  }

  const subToDelete = parsedAdd.task.sub_tasks.find((s) => s.title === 'Sub B').sub_task_id;
  const deleteSub = await client.callTool({
    name: 'delete_todo_sub',
    arguments: { master_task_id: 'task_mock001', sub_task_id: subToDelete },
  });
  if (deleteSub.isError) {
    throw new Error(`unexpected delete_todo_sub: ${deleteSub.content?.[0]?.text || ''}`);
  }

  const deleteMaster = await client.callTool({
    name: 'delete_todo_task',
    arguments: { master_task_id: 'task_mock001' },
  });
  const deleteMasterText = deleteMaster.content?.[0]?.text || '';
  if (deleteMaster.isError || !deleteMasterText.includes('"ok":true')) {
    throw new Error(`unexpected delete_todo_task: ${deleteMasterText}`);
  }

  // T8: complete without sub_task_id → master complete; idempotent; abandoned reject;
  // list/get/create status wire includes abandoned.
  resetPlanTaskStore();
  const masterCreate = await client.callTool({
    name: 'create_todo_task',
    arguments: { title: 'Master complete path' },
  });
  const masterCreateText = masterCreate.content?.[0]?.text || '';
  if (masterCreate.isError) {
    throw new Error(`unexpected master-complete create: ${masterCreateText}`);
  }
  const completeMaster = await client.callTool({
    name: 'complete_todo',
    arguments: { master_task_id: 'task_mock001' },
  });
  const completeMasterText = completeMaster.content?.[0]?.text || '';
  if (completeMaster.isError) {
    throw new Error(`unexpected complete_todo without sub_task_id: ${completeMasterText}`);
  }
  const completeMasterTask = JSON.parse(completeMasterText).task;
  if (completeMasterTask.status !== 'complete') {
    throw new Error(
      `complete_todo without sub_task_id must set master complete: ${completeMasterText}`,
    );
  }
  assertMasterStatusWire(completeMasterTask.status, 'complete_todo without sub');

  const completeIdempotent = await client.callTool({
    name: 'complete_todo',
    arguments: { master_task_id: 'task_mock001' },
  });
  const completeIdempotentText = completeIdempotent.content?.[0]?.text || '';
  if (completeIdempotent.isError) {
    throw new Error(`complete_todo on already-complete must be idempotent: ${completeIdempotentText}`);
  }
  if (JSON.parse(completeIdempotentText).task.status !== 'complete') {
    throw new Error(`idempotent complete_todo must keep status complete: ${completeIdempotentText}`);
  }

  seedMaster('task_abandoned', 'Abandoned master', 'abandoned');
  const listTri = await client.callTool({ name: 'list_todo_tasks', arguments: {} });
  const listTriText = listTri.content?.[0]?.text || '';
  if (listTri.isError) {
    throw new Error(`unexpected list after abandoned seed: ${listTriText}`);
  }
  const abandonedListed = JSON.parse(listTriText).find((t) => t.master_task_id === 'task_abandoned');
  if (!abandonedListed || abandonedListed.status !== 'abandoned') {
    throw new Error(`list_todo_tasks must read back abandoned status: ${listTriText}`);
  }
  assertMasterStatusWire(abandonedListed.status, 'list_todo_tasks abandoned');

  const getAbandoned = await client.callTool({
    name: 'get_todo_task',
    arguments: { id: 'task_abandoned' },
  });
  const getAbandonedText = getAbandoned.content?.[0]?.text || '';
  if (getAbandoned.isError) {
    throw new Error(`unexpected get abandoned: ${getAbandonedText}`);
  }
  if (JSON.parse(getAbandonedText).status !== 'abandoned') {
    throw new Error(`get_todo_task must read back abandoned status: ${getAbandonedText}`);
  }
  assertMasterStatusWire(JSON.parse(getAbandonedText).status, 'get_todo_task abandoned');

  const rejectAbandoned = await client.callTool({
    name: 'complete_todo',
    arguments: { master_task_id: 'task_abandoned' },
  });
  const rejectAbandonedText = rejectAbandoned.content?.[0]?.text || '';
  if (!rejectAbandoned.isError || !rejectAbandonedText.includes('master_abandoned')) {
    throw new Error(
      `complete_todo on abandoned must reject with master_abandoned, got: ${rejectAbandonedText}`,
    );
  }

  // T9: attachment tools proxyPost to T8 paths; not nested into get/list task responses.
  resetPlanTaskStore();
  planAttachmentHttpCalls.length = 0;
  const attachCreate = await client.callTool({
    name: 'create_todo_task',
    arguments: { title: 'Attach master' },
  });
  if (attachCreate.isError) {
    throw new Error(`unexpected attach create: ${attachCreate.content?.[0]?.text || ''}`);
  }

  const addAttach = await client.callTool({
    name: 'add_todo_attachment',
    arguments: {
      master_task_id: 'task_mock001',
      file_name: 'notes.md',
      content: '# Notes\n',
    },
  });
  const addAttachText = addAttach.content?.[0]?.text || '';
  if (addAttach.isError || !addAttachText.includes('notes.md')) {
    throw new Error(`unexpected add_todo_attachment: ${addAttachText}`);
  }

  const listAttach = await client.callTool({
    name: 'list_todo_attachments',
    arguments: { master_task_id: 'task_mock001' },
  });
  const listAttachText = listAttach.content?.[0]?.text || '';
  if (listAttach.isError || !listAttachText.includes('notes.md')) {
    throw new Error(`unexpected list_todo_attachments: ${listAttachText}`);
  }

  const getAttach = await client.callTool({
    name: 'get_todo_attachment',
    arguments: { master_task_id: 'task_mock001', file_name: 'notes.md' },
  });
  const getAttachText = getAttach.content?.[0]?.text || '';
  if (getAttach.isError || !getAttachText.includes('# Notes')) {
    throw new Error(`unexpected get_todo_attachment: ${getAttachText}`);
  }

  const updateAttach = await client.callTool({
    name: 'update_todo_attachment',
    arguments: {
      master_task_id: 'task_mock001',
      file_name: 'notes.md',
      content: 'updated body',
    },
  });
  const updateAttachText = updateAttach.content?.[0]?.text || '';
  if (updateAttach.isError || !updateAttachText.includes('"ok":true')) {
    throw new Error(`unexpected update_todo_attachment: ${updateAttachText}`);
  }

  const rereadAttach = await client.callTool({
    name: 'get_todo_attachment',
    arguments: { master_task_id: 'task_mock001', file_name: 'notes.md' },
  });
  const rereadAttachText = rereadAttach.content?.[0]?.text || '';
  if (rereadAttach.isError || !rereadAttachText.includes('updated body')) {
    throw new Error(`unexpected get_todo_attachment after update: ${rereadAttachText}`);
  }

  const expectedPaths = [
    ATTACHMENT_TOOL_HTTP_PATHS.add_todo_attachment,
    ATTACHMENT_TOOL_HTTP_PATHS.list_todo_attachments,
    ATTACHMENT_TOOL_HTTP_PATHS.get_todo_attachment,
    ATTACHMENT_TOOL_HTTP_PATHS.update_todo_attachment,
    ATTACHMENT_TOOL_HTTP_PATHS.get_todo_attachment,
  ];
  const actualPaths = planAttachmentHttpCalls.map((c) => c.path);
  if (JSON.stringify(actualPaths) !== JSON.stringify(expectedPaths)) {
    throw new Error(
      `attachment tool HTTP paths mismatch: expected ${JSON.stringify(expectedPaths)}, got ${JSON.stringify(actualPaths)}`,
    );
  }

  const getTaskAfterAttach = await client.callTool({
    name: 'get_todo_task',
    arguments: { id: 'task_mock001' },
  });
  const getTaskAfterAttachText = getTaskAfterAttach.content?.[0]?.text || '';
  if (getTaskAfterAttach.isError) {
    throw new Error(`unexpected get_todo_task after attach: ${getTaskAfterAttachText}`);
  }
  const getTaskParsed = JSON.parse(getTaskAfterAttachText);
  if (Object.prototype.hasOwnProperty.call(getTaskParsed, 'attachments')) {
    throw new Error('get_todo_task must not embed attachments');
  }

  const listTasksAfterAttach = await client.callTool({ name: 'list_todo_tasks', arguments: {} });
  const listTasksAfterAttachText = listTasksAfterAttach.content?.[0]?.text || '';
  if (listTasksAfterAttach.isError) {
    throw new Error(`unexpected list_todo_tasks after attach: ${listTasksAfterAttachText}`);
  }
  const listTasksParsed = JSON.parse(listTasksAfterAttachText);
  if (!Array.isArray(listTasksParsed) || listTasksParsed.some((t) => Object.prototype.hasOwnProperty.call(t, 'attachments'))) {
    throw new Error('list_todo_tasks must not embed attachments');
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

const REPO_ROOT = path.join(PKG_ROOT, '..', '..');

/**
 * AC1 — Host registry/readiness inject URL for Binding key todo_task is
 * http://127.0.0.1:<mcp_port>/mcp/todo_task (key≡scene_slot; no bare /mcp overwrite).
 */
async function assertAc1AppBindingInjectUrl() {
  const registrySrc = fs.readFileSync(
    path.join(REPO_ROOT, 'src-tauri/src/services/mcp_server_registry.rs'),
    'utf8',
  );
  const readinessSrc = fs.readFileSync(
    path.join(REPO_ROOT, 'src-tauri/src/services/mcp_endpoint_readiness.rs'),
    'utf8',
  );
  const readinessTests = fs.readFileSync(
    path.join(REPO_ROOT, 'src-tauri/src/unit-tests/services/mcp_endpoint_readiness_tests.rs'),
    'utf8',
  );

  if (!registrySrc.includes('SEEDED_BUSINESS_KEY: &str = "todo_task"')) {
    throw new Error('AC1: registry seed key must be todo_task');
  }
  if (!registrySrc.includes('/mcp/{}') || !registrySrc.includes('SEEDED_BUSINESS_KEY')) {
    throw new Error('AC1: registry seed URL must use /mcp/<SEEDED_BUSINESS_KEY>');
  }
  // Seeded URL shape: http://127.0.0.1:{port}/mcp/{key}
  if (!/format!\(\s*"http:\/\/127\.0\.0\.1:\{\}\/mcp\/\{\}"/.test(registrySrc)) {
    throw new Error(
      'AC1: registry seeded transport must format http://127.0.0.1:{}/mcp/{}',
    );
  }
  if (!readinessSrc.includes('format!("http://127.0.0.1:{mcp_port}/mcp/{key}")')) {
    throw new Error(
      'AC1: readiness must inject http://127.0.0.1:{mcp_port}/mcp/{key} (not bare /mcp)',
    );
  }
  if (readinessSrc.includes('format!("http://127.0.0.1:{mcp_port}/mcp")')) {
    throw new Error('AC1: readiness must not overwrite with bare /mcp');
  }
  if (!readinessTests.includes('/mcp/{SEEDED_BUSINESS_KEY}')) {
    throw new Error('AC1: readiness tests must observe inject URL .../mcp/todo_task');
  }
  if (!readinessTests.includes('ready_transports_inject_mcp_key_path_not_bare_mcp')) {
    throw new Error('AC1: missing readiness test ready_transports_inject_mcp_key_path_not_bare_mcp');
  }
}

/**
 * AC2–AC5 + dual-slot boundary — one sidecar session:
 * tools/list isolation/stability, unknown hard-reject, HTTP success (A1 close).
 */
async function assertAc2ThroughAc5Runtime(workbenchUrl) {
  const mcpPort = await ephemeralPort();
  const sidecar = spawnSidecar(mcpPort, workbenchUrl);
  const ready = await waitFor(`http://127.0.0.1:${mcpPort}/health`);
  if (!ready) {
    sidecar.kill('SIGTERM');
    throw new Error('dual-channel acceptance: sidecar health timeout');
  }
  try {
    // AC2 / A1 — todo_task tools/list
    const todoA = await connectAndListToolNames(mcpPort, 'todo_task');
    assertIncludesAll(todoA, TODO_SURFACE, 'AC2 /mcp/todo_task');
    assertNoneOf(todoA, CORPUS_TOOLS, 'AC2 /mcp/todo_task');
    const todoB = await connectAndListToolNames(mcpPort, 'todo_task');
    if (JSON.stringify(todoA) !== JSON.stringify(todoB)) {
      throw new Error(`dual-slot stability: todo_task jitter ${todoA} vs ${todoB}`);
    }

    // AC3 / A1 — cursor_ide tools/list
    const ideA = await connectAndListToolNames(mcpPort, 'cursor_ide');
    assertIncludesAll(ideA, CORPUS_TOOLS, 'AC3 /mcp/cursor_ide');
    assertNoneOf(ideA, TODO_SURFACE, 'AC3 /mcp/cursor_ide');
    const ideB = await connectAndListToolNames(mcpPort, 'cursor_ide');
    if (JSON.stringify(ideA) !== JSON.stringify(ideB)) {
      throw new Error(`dual-slot stability: cursor_ide jitter ${ideA} vs ${ideB}`);
    }
    if (JSON.stringify(todoA) === JSON.stringify(ideA)) {
      throw new Error('dual-slot: todo_task and cursor_ide tools/list must be distinguishable');
    }

    // AC4 — unknown hard-fail
    const unknown = await tryConnectAndListToolNames(mcpPort, '/mcp/__unknown__');
    if (unknown.ok) {
      throw new Error(
        `AC4: unknown scene_slot must hard-fail; got tools: ${(unknown.names || []).join(', ')}`,
      );
    }

    // AC5 — registered slot → Workbench HTTP success
    const transport = new StreamableHTTPClientTransport(
      new URL(`http://127.0.0.1:${mcpPort}/mcp/todo_task`),
    );
    const client = new Client({ name: 'knowledge-mcp-ac5', version: '0.1.0' });
    await client.connect(transport);
    const result = await client.callTool({ name: 'list_todo_tasks', arguments: {} });
    if (result.isError) {
      throw new Error(
        `AC5: list_todo_tasks via Workbench HTTP must succeed: ${result.content?.[0]?.text || ''}`,
      );
    }
    await client.close();
  } finally {
    sidecar.kill('SIGTERM');
    await sleep(200);
  }
}

/**
 * AC6 — App Binding remains key-only; IDE channel is mcp.json manual only
 * (docs + Binding consumer + Host registry key seed).
 */
async function assertAc6KeyOnlyBindingAndIdeMcpJsonOnly() {
  const bindingSrc = fs.readFileSync(
    path.join(REPO_ROOT, 'frontend/js/plan-task/todos-binding.js'),
    'utf8',
  );
  const docSrc = fs.readFileSync(path.join(REPO_ROOT, 'docs/knowledge-mcp.md'), 'utf8');
  const ideUrlLiteral = 'http://127.0.0.1:<mcp_port>/mcp/cursor_ide';
  const appUrlLiteral = 'http://127.0.0.1:<mcp_port>/mcp/todo_task';

  if (!bindingSrc.includes("TODOS_BUSINESS_KEY = 'todo_task'")) {
    throw new Error('AC6: Binding consumer must seed key todo_task');
  }
  if (!bindingSrc.includes('return { key: TODOS_BUSINESS_KEY }')) {
    throw new Error('AC6: assembleTodosBindingBody must be key-only');
  }
  const assembleIdx = bindingSrc.indexOf('function assembleTodosBindingBody');
  if (assembleIdx < 0) {
    throw new Error('AC6: missing assembleTodosBindingBody');
  }
  const assembleSlice = bindingSrc.slice(assembleIdx, assembleIdx + 280);
  if (/\btools\s*:/.test(assembleSlice) || /\bprompt\s*:/.test(assembleSlice)) {
    throw new Error('AC6: Binding assemble must not carry tools/prompt (key-only)');
  }

  if (!docSrc.includes(ideUrlLiteral)) {
    throw new Error(`AC6: docs must document IDE URL ${ideUrlLiteral}`);
  }
  if (!docSrc.includes(appUrlLiteral)) {
    throw new Error(`AC6: docs must document App Binding URL ${appUrlLiteral}`);
  }
  if (!docSrc.includes('mcp.json')) {
    throw new Error('AC6: docs must mention mcp.json for IDE channel');
  }
  if (!docSrc.includes('不经') || !docSrc.includes('Binding')) {
    throw new Error('AC6: docs must state IDE channel does not use Binding');
  }
}

/**
 * t4 / T8 — Dual-channel + unknown-slot E2E acceptance (AC1–AC6, A1).
 * A1 close observation = tools/list success on both registered path slots (AC2+AC3).
 */
async function testDualChannelAcceptanceAc1ToAc6(workbenchUrl) {
  await assertAc1AppBindingInjectUrl();
  await assertAc2ThroughAc5Runtime(workbenchUrl);
  await assertAc6KeyOnlyBindingAndIdeMcpJsonOnly();
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
  await sleep(200);

  await testSceneSlotPathRouting(workbenchUrl);
  console.log('scene_slot path routing: OK');

  // Keep mock HTTP up for dual-channel acceptance HTTP success path (AC5).
  await testDualChannelAcceptanceAc1ToAc6(workbenchUrl);
  console.log('dual-channel AC1–AC6 acceptance: OK');

  mockServer.close();
  await sleep(200);

  await testUnreachableWorkbenchHttp();
  console.log('unreachable Workbench HTTP tools/call isError: OK');

  await testMissingWorkbenchUrl();
  console.log('missing WORKBENCH_HTTP_URL exits: OK');

  console.log('\nknowledge-mcp verify PASSED');
}

main().catch((err) => {
  console.error('knowledge-mcp verify FAILED:', err);
  process.exit(1);
});
