#!/usr/bin/env node
/**
 * E2E: todo MCP tools against Host MCP URL (default http://127.0.0.1:9876).
 * Env: MCP_PORT / HOST_MCP_PORT (default 9876); E2E_TODO_TASKS_TASKS_DIR (optional)
 * Does not spawn the archived Node MCP package — Host is runtime SSOT.
 */
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

/** @typedef {{ master_task_id: string, todo_md?: unknown, migration_error?: unknown }} PlanMaster */

export function assertMasterPlanMdFields(master, label) {
  if (master == null || typeof master !== 'object') {
    throw new Error(`${label}: expected object, got ${typeof master}`);
  }
  if (!Object.prototype.hasOwnProperty.call(master, 'todo_md')) {
    throw new Error(`${label}: missing top-level todo_md field`);
  }
  if (typeof master.todo_md !== 'string') {
    throw new Error(`${label}: todo_md must be a string, got ${typeof master.todo_md}`);
  }
  if (!Object.prototype.hasOwnProperty.call(master, 'migration_error')) {
    throw new Error(`${label}: missing top-level migration_error field`);
  }
  if (typeof master.migration_error !== 'boolean') {
    throw new Error(`${label}: migration_error must be a boolean, got ${typeof master.migration_error}`);
  }
}

export function assertPlanMdMatchesDisk(planMd, diskContent, label) {
  if (planMd !== diskContent) {
    throw new Error(
      `${label}: todo_md does not match disk todo.md (len ${planMd.length} vs ${diskContent.length})`,
    );
  }
}

export function readPlanMdFromDisk(tasksDir, masterId) {
  const planPath = path.join(tasksDir, masterId, 'todo.md');
  try {
    return fs.readFileSync(planPath, 'utf8');
  } catch (err) {
    if (err && typeof err === 'object' && 'code' in err && err.code === 'ENOENT') {
      return '';
    }
    throw new Error(`failed to read todo.md for ${masterId}: ${err}`);
  }
}

export function writePlanMdToDisk(tasksDir, masterId, content) {
  const taskDir = path.join(tasksDir, masterId);
  fs.mkdirSync(taskDir, { recursive: true });
  fs.writeFileSync(path.join(taskDir, 'todo.md'), content, 'utf8');
}

const E2E_REPO_ROOT = path.join(path.dirname(fileURLToPath(import.meta.url)), '..');

const NOTES_TOOLS_E2E = [
  'get_notes_catalog',
  'get_notes_files',
  'create_note',
];

/**
 * t4 / T8 — Dual-channel e2e contract (static AC6 + URL shape).
 * Live AC2/AC3/AC4/A1 probes run after MCP connect when sidecar is up.
 */
export function assertDualChannelE2eContract() {
  const docPath = path.join(E2E_REPO_ROOT, 'docs', 'knowledge-mcp.md');
  const bindingPath = path.join(E2E_REPO_ROOT, 'frontend', 'js', 'todo-task', 'binding.js');
  if (!fs.existsSync(docPath)) {
    throw new Error('dual-channel e2e: missing docs/knowledge-mcp.md');
  }
  if (!fs.existsSync(bindingPath)) {
    throw new Error('dual-channel e2e: missing binding.js');
  }
  const doc = fs.readFileSync(docPath, 'utf8');
  const binding = fs.readFileSync(bindingPath, 'utf8');
  if (!doc.includes('http://127.0.0.1:<mcp_port>/mcp/cursor_ide')) {
    throw new Error('dual-channel e2e: IDE URL convention missing from docs');
  }
  if (!doc.includes('http://127.0.0.1:<mcp_port>/mcp/workbench')) {
    throw new Error('dual-channel e2e: App Binding URL convention missing from docs');
  }
  if (!doc.includes('mcp.json')) {
    throw new Error('dual-channel e2e: docs must mention mcp.json');
  }
  if (!binding.includes("WORKBENCH_BUSINESS_KEY = 'workbench'")) {
    throw new Error('dual-channel e2e: Binding key must be workbench');
  }
  if (!binding.includes('key: WORKBENCH_BUSINESS_KEY')) {
    throw new Error('dual-channel e2e: Binding must remain key-only');
  }
}

async function listToolNamesOnSlot(mcpPort, sceneSlot, clientName) {
  const transport = new StreamableHTTPClientTransport(
    new URL(`http://127.0.0.1:${mcpPort}/mcp/${encodeURIComponent(sceneSlot)}`),
  );
  const client = new Client({ name: clientName, version: '0.3.0' });
  await client.connect(transport);
  try {
    const tools = await client.listTools();
    return tools.tools.map((t) => t.name);
  } finally {
    await client.close();
  }
}

/** Live A1/AC2/AC3/AC4 observation: path URL tools/list on both slots; unknown hard-fail. */
async function runDualChannelLiveProbes(mcpPort) {
  const workbenchNames = await listToolNamesOnSlot(mcpPort, 'workbench', 'todo-task-mcp-e2e-dual-workbench');
  for (const tool of EQUIVALENCE_TODO_TOOLS) {
    if (!workbenchNames.includes(tool)) {
      throw new Error(`dual-channel AC2: workbench missing ${tool}`);
    }
  }
  for (const tool of NOTES_TOOLS_E2E) {
    if (!workbenchNames.includes(tool)) {
      throw new Error(`dual-channel AC2: workbench missing ${tool}`);
    }
  }
  if (workbenchNames.includes('get_notes_selection')) {
    throw new Error('dual-channel AC2: workbench must not expose get_notes_selection');
  }

  const ideNames = await listToolNamesOnSlot(mcpPort, 'cursor_ide', 'todo-task-mcp-e2e-dual-ide');
  for (const tool of NOTES_TOOLS_E2E) {
    if (!ideNames.includes(tool)) {
      throw new Error(`dual-channel AC3: cursor_ide missing ${tool}`);
    }
  }
  for (const tool of EQUIVALENCE_TODO_TOOLS) {
    if (!ideNames.includes(tool)) {
      throw new Error(`dual-channel AC3: cursor_ide missing ${tool}`);
    }
  }

  // AC4 — unknown slot hard-fail (connect/listTools must not succeed)
  try {
    const names = await listToolNamesOnSlot(mcpPort, '__unknown__', 'todo-task-mcp-e2e-dual-unknown');
    throw new Error(`dual-channel AC4: unknown slot must hard-fail; got ${names.join(', ')}`);
  } catch (err) {
    if (String(err).includes('dual-channel AC4:')) throw err;
    // connect/listTools failure is the expected hard-reject
  }
}

function runSelfTest() {
  assertMasterPlanMdFields({ master_task_id: 'x', todo_md: '', migration_error: false }, 'ok');
  try {
    assertMasterPlanMdFields({ master_task_id: 'x' }, 'bad');
    throw new Error('expected missing todo_md to throw');
  } catch (err) {
    if (!String(err).includes('missing top-level todo_md')) {
      throw err;
    }
  }
  assertPlanMdMatchesDisk('hello', 'hello', 'match');
  assertDualChannelE2eContract();
  console.log('todo-task-mcp-e2e self-test PASSED');
}

if (process.argv.includes('--self-test')) {
  runSelfTest();
  process.exit(0);
}

const mcpPort = Number(process.env.MCP_PORT || process.env.HOST_MCP_PORT || 9876);
if (!Number.isFinite(mcpPort) || mcpPort <= 0) {
  console.error('todo-task-mcp-e2e: MCP_PORT / HOST_MCP_PORT must be a positive port (default 9876)');
  process.exit(1);
}

const tasksDir = process.env.E2E_TODO_TASKS_TASKS_DIR?.trim() || '';

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

const TODO_TOOLS = EQUIVALENCE_TODO_TOOLS;

/** Breaking rename: old MCP tool name must not remain registered (T8 / AC5 / R1). */
const FORBIDDEN_PLAN_TOOLS = [
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

function assertMasterStatusWire(status, label) {
  if (typeof status !== 'string' || !MASTER_STATUS_WIRE.includes(status)) {
    throw new Error(
      `${label}: status must be one of ${MASTER_STATUS_WIRE.join('|')}, got ${JSON.stringify(status)}`,
    );
  }
}

const transport = new StreamableHTTPClientTransport(
  new URL(`http://127.0.0.1:${mcpPort}/mcp/workbench`),
);
const client = new Client({ name: 'todo-task-mcp-e2e', version: '0.3.0' });

try {
  await client.connect(transport);
} catch (err) {
  console.error(`todo-task-mcp-e2e: MCP connect failed (is Host MCP up on http://127.0.0.1:${mcpPort}?): ${err}`);
  process.exit(1);
}

const tools = await client.listTools();
const names = tools.tools.map((t) => t.name);
for (const tool of TODO_TOOLS) {
  if (!names.includes(tool)) {
    throw new Error(`${tool} missing from tools: ${names.join(', ')}`);
  }
}
for (const tool of FORBIDDEN_PLAN_TOOLS) {
  if (names.includes(tool)) {
    throw new Error(`forbidden plan_* tool still registered: ${tool}`);
  }
}

// t4 / T8 — dual-channel live probes (AC2/AC3/AC4 + A1 tools/list observation)
assertDualChannelE2eContract();
await runDualChannelLiveProbes(mcpPort);
console.log('todo-task-mcp-e2e dual-channel probes: OK');

function toolText(result) {
  return result.content?.[0]?.text || '';
}

function parseJson(text) {
  return JSON.parse(text);
}

async function callTodoTool(name, args = {}) {
  const result = await client.callTool({ name, arguments: args });
  const text = toolText(result);
  if (result.isError) {
    throw new Error(`${name} failed: ${text}`);
  }
  return parseJson(text);
}

function findMaster(list, masterId, label) {
  const row = list.find((t) => t.master_task_id === masterId);
  if (!row) {
    throw new Error(`${label} missing master ${masterId}`);
  }
  return row;
}

const createResult = await client.callTool({
  name: 'create_todo_task',
  arguments: { title: 'MCP E2E', todo_md: '## E2E plan\n\nBody' },
});
const createText = toolText(createResult);
if (createResult.isError) {
  throw new Error(
    `create_todo_task failed (Workbench HTTP may be down): ${createText}`,
  );
}
const created = parseJson(createText);
const masterId = created.master_task_id;
if (!masterId) {
  throw new Error(`unexpected create_todo_task response: ${createText}`);
}
const createStatus = created.task?.status ?? created.status;
if (createStatus !== 'incomplete') {
  throw new Error(`create_todo_task status must be incomplete, got: ${createText}`);
}
assertMasterStatusWire(createStatus, 'create_todo_task');

const addA = await callTodoTool('add_todo_sub', {
  master_task_id: masterId,
  title: 'Sub A',
});
const addB = await callTodoTool('add_todo_sub', {
  master_task_id: masterId,
  title: 'Sub B',
});
const subA = addA.task.sub_tasks.find((s) => s.title === 'Sub A')?.sub_task_id;
const subB = addB.task.sub_tasks.find((s) => s.title === 'Sub B')?.sub_task_id;
if (!subA || !subB) {
  throw new Error(`unexpected add_todo_sub response after create: ${JSON.stringify({ addA, addB })}`);
}

const listResult = await client.callTool({ name: 'list_todo_tasks', arguments: {} });
const listText = toolText(listResult);
if (listResult.isError) {
  throw new Error(`list_todo_tasks failed (Workbench HTTP may be down): ${listText}`);
}
const listBody = parseJson(listText);
if (!Array.isArray(listBody)) {
  throw new Error(`list_todo_tasks expected JSON array: ${listText}`);
}
const listMaster = listBody.find((t) => t.master_task_id === masterId);
if (!listMaster) {
  throw new Error(`list_todo_tasks missing created master ${masterId}: ${listText}`);
}
assertMasterPlanMdFields(listMaster, 'list_todo_tasks');
assertMasterStatusWire(listMaster.status, 'list_todo_tasks');
const expectedInitialPlanMd = '## E2E plan\n\nBody';
const diskBefore = tasksDir ? readPlanMdFromDisk(tasksDir, masterId) : expectedInitialPlanMd;
assertPlanMdMatchesDisk(listMaster.todo_md, diskBefore, 'list_todo_tasks');
if (listMaster.todo_md !== expectedInitialPlanMd) {
  throw new Error(
    `list_todo_tasks todo_md should match create todo_md, got: ${JSON.stringify(listMaster.todo_md)}`,
  );
}

const getResult = await client.callTool({
  name: 'get_todo_task',
  arguments: { id: masterId },
});
const getText = toolText(getResult);
if (getResult.isError) {
  throw new Error(`get_todo_task failed: ${getText}`);
}
const getBody = parseJson(getText);
assertMasterPlanMdFields(getBody, 'get_todo_task');
assertMasterStatusWire(getBody.status, 'get_todo_task');
if (getBody.todo_md !== listMaster.todo_md) {
  throw new Error('get_todo_task todo_md must match list_todo_tasks for same id');
}
assertPlanMdMatchesDisk(getBody.todo_md, diskBefore, 'get_todo_task');

const updateTitle = await callTodoTool('update_todo_task', {
  master_task_id: masterId,
  title: 'MCP E2E Renamed',
});
if (updateTitle.task?.title !== 'MCP E2E Renamed') {
  throw new Error(`update_todo_task title-only failed: ${JSON.stringify(updateTitle)}`);
}
if (updateTitle.task?.todo_md !== getBody.todo_md) {
  throw new Error('update_todo_task title-only must preserve todo_md');
}

const updateBody = await callTodoTool('update_todo_task', {
  master_task_id: masterId,
  todo_md: '## E2E plan\n\nUpdated body',
});
if (updateBody.task?.title !== 'MCP E2E Renamed') {
  throw new Error('update_todo_task body-only must preserve title');
}
if (updateBody.task?.todo_md !== '## E2E plan\n\nUpdated body') {
  throw new Error(`update_todo_task body-only failed: ${JSON.stringify(updateBody)}`);
}
assertMasterPlanMdFields(updateBody.task, 'update_todo_task');
if (tasksDir) {
  assertPlanMdMatchesDisk(
    updateBody.task.todo_md,
    readPlanMdFromDisk(tasksDir, masterId),
    'update_todo_task after body write',
  );
}

const updateNeither = await client.callTool({
  name: 'update_todo_task',
  arguments: { master_task_id: masterId },
});
if (!updateNeither.isError) {
  throw new Error('expected update_todo_task error when neither title nor todo_md provided');
}

if (tasksDir) {
  const fixtureContent = '# MCP E2E plan.md\n\nRound-trip fixture paragraph.\n';
  writePlanMdToDisk(tasksDir, masterId, fixtureContent);
  const listRow = findMaster(
    await callTodoTool('list_todo_tasks'),
    masterId,
    'list_todo_tasks after disk write',
  );
  assertPlanMdMatchesDisk(listRow.todo_md, fixtureContent, 'list_todo_tasks after disk write');
  const getAfterWrite = await callTodoTool('get_todo_task', { id: masterId });
  assertPlanMdMatchesDisk(getAfterWrite.todo_md, fixtureContent, 'get_todo_task after disk write');

  writePlanMdToDisk(tasksDir, masterId, '');
  const listEmptyRow = findMaster(
    await callTodoTool('list_todo_tasks'),
    masterId,
    'list_todo_tasks empty plan.md',
  );
  assertPlanMdMatchesDisk(listEmptyRow.todo_md, '', 'list_todo_tasks empty plan.md');
}

const unknownGet = await client.callTool({
  name: 'get_todo_task',
  arguments: { id: '00000000000000000000000000000000' },
});
if (!unknownGet.isError) {
  throw new Error('expected get_todo_task error for unknown id');
}

const addSubResult = await client.callTool({
  name: 'add_todo_sub',
  arguments: { master_task_id: masterId, title: 'Sub C' },
});
const addSubText = toolText(addSubResult);
if (addSubResult.isError || !addSubText.includes('Sub C')) {
  throw new Error(`add_todo_sub failed: ${addSubText}`);
}

// t5 / AC2 — create-with-content round-trip
const addWithContentResult = await client.callTool({
  name: 'add_todo_sub',
  arguments: {
    master_task_id: masterId,
    title: 'Sub with content',
    content: 'e2e content body',
  },
});
const addWithContentText = toolText(addWithContentResult);
if (addWithContentResult.isError || !addWithContentText.includes('e2e content body')) {
  throw new Error(`add_todo_sub with content failed: ${addWithContentText}`);
}
const contentSubId = parseJson(addWithContentText).task.sub_tasks.find(
  (s) => s.title === 'Sub with content',
)?.sub_task_id;
if (!contentSubId) {
  throw new Error(`add_todo_sub with content missing sub id: ${addWithContentText}`);
}

// t5 / AC3 — update_todo_sub modify / clear (content: '')
const updateSubSet = await client.callTool({
  name: 'update_todo_sub',
  arguments: {
    master_task_id: masterId,
    sub_task_id: contentSubId,
    content: 'e2e content v2',
  },
});
const updateSubSetText = toolText(updateSubSet);
if (updateSubSet.isError || !updateSubSetText.includes('e2e content v2')) {
  throw new Error(`update_todo_sub set content failed: ${updateSubSetText}`);
}

const updateSubClear = await client.callTool({
  name: 'update_todo_sub',
  arguments: {
    master_task_id: masterId,
    sub_task_id: contentSubId,
    content: '',
  },
});
const updateSubClearText = toolText(updateSubClear);
if (updateSubClear.isError) {
  throw new Error(`update_todo_sub clear content failed: ${updateSubClearText}`);
}
const clearedSub = parseJson(updateSubClearText).task.sub_tasks.find(
  (s) => s.sub_task_id === contentSubId,
);
if (!clearedSub || (clearedSub.content != null && clearedSub.content !== '')) {
  throw new Error(`update_todo_sub content: '' must clear: ${updateSubClearText}`);
}

const completeResult = await client.callTool({
  name: 'complete_todo',
  arguments: { master_task_id: masterId, sub_task_id: subA },
});
const completeText = toolText(completeResult);
if (completeResult.isError || !completeText.includes('complete')) {
  throw new Error(`complete_todo with sub_task_id failed: ${completeText}`);
}
const afterSubComplete = parseJson(completeText).task;
if (afterSubComplete.status !== listMaster.status) {
  throw new Error(
    `complete_todo with sub_task_id must not rewrite master status, got: ${completeText}`,
  );
}

const archiveId = '138700959e5ddb1260c69e9e18169ac4';
const linkResult = await client.callTool({
  name: 'link_todo_archive',
  arguments: { master_task_id: masterId, sub_task_id: subA, archive_id: archiveId },
});
const linkText = toolText(linkResult);
if (linkResult.isError || !linkText.includes(archiveId)) {
  throw new Error(`link_todo_archive failed: ${linkText}`);
}

// T10 / AC-等价: attachment quartet must be exercisable (not only create/list/get).
const attachStageDir = fs.mkdtempSync(path.join(os.tmpdir(), 'todo-attach-e2e-'));
const addAttachSource = path.join(attachStageDir, 'e2e-notes.md');
fs.writeFileSync(addAttachSource, '# E2E attachment\n', 'utf8');
const addAttachResult = await client.callTool({
  name: 'add_todo_attachment',
  arguments: {
    master_task_id: masterId,
    source_path: addAttachSource,
  },
});
const addAttachText = toolText(addAttachResult);
if (addAttachResult.isError || !addAttachText.includes('e2e-notes.md')) {
  throw new Error(`add_todo_attachment failed: ${addAttachText}`);
}

const listAttachResult = await client.callTool({
  name: 'list_todo_attachments',
  arguments: { master_task_id: masterId },
});
const listAttachText = toolText(listAttachResult);
if (listAttachResult.isError || !listAttachText.includes('e2e-notes.md')) {
  throw new Error(`list_todo_attachments failed: ${listAttachText}`);
}

const getAttachResult = await client.callTool({
  name: 'get_todo_attachment',
  arguments: { master_task_id: masterId, file_name: 'e2e-notes.md' },
});
const getAttachText = toolText(getAttachResult);
if (getAttachResult.isError || !getAttachText.includes('# E2E attachment')) {
  throw new Error(`get_todo_attachment failed: ${getAttachText}`);
}

const updateAttachSource = path.join(attachStageDir, 'e2e-notes-updated.md');
fs.writeFileSync(updateAttachSource, 'updated e2e attachment', 'utf8');
const updateAttachResult = await client.callTool({
  name: 'update_todo_attachment',
  arguments: {
    master_task_id: masterId,
    file_name: 'e2e-notes.md',
    source_path: updateAttachSource,
  },
});
const updateAttachText = toolText(updateAttachResult);
if (updateAttachResult.isError || !updateAttachText.includes('"ok":true')) {
  throw new Error(`update_todo_attachment failed: ${updateAttachText}`);
}

const rereadAttachResult = await client.callTool({
  name: 'get_todo_attachment',
  arguments: { master_task_id: masterId, file_name: 'e2e-notes.md' },
});
const rereadAttachText = toolText(rereadAttachResult);
if (rereadAttachResult.isError || !rereadAttachText.includes('updated e2e attachment')) {
  throw new Error(`get_todo_attachment after update failed: ${rereadAttachText}`);
}

const deleteSubResult = await client.callTool({
  name: 'delete_todo_sub',
  arguments: { master_task_id: masterId, sub_task_id: subB },
});
if (deleteSubResult.isError) {
  throw new Error(`delete_todo_sub failed: ${toolText(deleteSubResult)}`);
}

// T8: complete without sub_task_id → master complete; idempotent when already complete.
const completeMasterResult = await client.callTool({
  name: 'complete_todo',
  arguments: { master_task_id: masterId },
});
const completeMasterText = toolText(completeMasterResult);
if (completeMasterResult.isError) {
  throw new Error(`complete_todo without sub_task_id failed: ${completeMasterText}`);
}
const completeMasterTask = parseJson(completeMasterText).task;
if (completeMasterTask.status !== 'complete') {
  throw new Error(
    `complete_todo without sub_task_id must set master complete: ${completeMasterText}`,
  );
}
assertMasterStatusWire(completeMasterTask.status, 'complete_todo without sub');

const completeIdempotent = await client.callTool({
  name: 'complete_todo',
  arguments: { master_task_id: masterId },
});
const completeIdempotentText = toolText(completeIdempotent);
if (completeIdempotent.isError) {
  throw new Error(`complete_todo on already-complete must be idempotent: ${completeIdempotentText}`);
}
if (parseJson(completeIdempotentText).task.status !== 'complete') {
  throw new Error(`idempotent complete_todo must keep status complete: ${completeIdempotentText}`);
}

// Optional: abandoned reject via host set-status when Workbench HTTP is reachable.
const workbenchUrl = (process.env.WORKBENCH_HTTP_URL || '').trim().replace(/\/$/, '');
if (workbenchUrl) {
  const abandonRes = await fetch(`${workbenchUrl}/api/todo-task-set-status`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ master_task_id: masterId, status: 'abandoned' }),
  });
  const abandonText = await abandonRes.text();
  if (!abandonRes.ok) {
    throw new Error(`set-status abandoned failed: HTTP ${abandonRes.status} ${abandonText}`);
  }
  const getAbandoned = await callTodoTool('get_todo_task', { id: masterId });
  if (getAbandoned.status !== 'abandoned') {
    throw new Error(`get_todo_task must read back abandoned after set-status: ${JSON.stringify(getAbandoned)}`);
  }
  assertMasterStatusWire(getAbandoned.status, 'get_todo_task abandoned');
  const rejectAbandoned = await client.callTool({
    name: 'complete_todo',
    arguments: { master_task_id: masterId },
  });
  const rejectAbandonedText = toolText(rejectAbandoned);
  if (!rejectAbandoned.isError || !rejectAbandonedText.includes('master_abandoned')) {
    throw new Error(
      `complete_todo on abandoned must reject with master_abandoned, got: ${rejectAbandonedText}`,
    );
  }
  // Restore complete so cleanup delete remains valid against host policy.
  const restoreRes = await fetch(`${workbenchUrl}/api/todo-task-set-status`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ master_task_id: masterId, status: 'complete' }),
  });
  if (!restoreRes.ok) {
    throw new Error(`set-status restore complete failed: HTTP ${restoreRes.status}`);
  }
} else {
  console.log(
    'todo-task-mcp-e2e: WORKBENCH_HTTP_URL unset; skipped abandoned reject/readback via set-status',
  );
}

const deleteMasterResult = await client.callTool({
  name: 'delete_todo_task',
  arguments: { master_task_id: masterId },
});
const deleteMasterText = toolText(deleteMasterResult);
if (deleteMasterResult.isError || !deleteMasterText.includes('"ok":true')) {
  throw new Error(`delete_todo_task failed: ${deleteMasterText}`);
}

const invalid = await client.callTool({
  name: 'create_todo_task',
  arguments: {
    title:
      'one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twentyone',
  },
});
if (!invalid.isError) {
  throw new Error('expected validation error for title over 20 words');
}

await client.close();
console.log('todo-task-mcp-e2e PASSED');
