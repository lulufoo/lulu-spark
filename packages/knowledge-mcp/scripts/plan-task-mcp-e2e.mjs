#!/usr/bin/env node
/**
 * E2E: plan MCP tools against live sidecar + Workbench HTTP.
 * Env: MCP_PORT (required); E2E_PLAN_TASKS_TASKS_DIR (optional, for plan.md disk round-trip)
 */
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js';
import fs from 'node:fs';
import path from 'node:path';

/** @typedef {{ master_task_id: string, plan_md?: unknown, migration_error?: unknown }} PlanMaster */

export function assertMasterPlanMdFields(master, label) {
  if (master == null || typeof master !== 'object') {
    throw new Error(`${label}: expected object, got ${typeof master}`);
  }
  if (!Object.prototype.hasOwnProperty.call(master, 'plan_md')) {
    throw new Error(`${label}: missing top-level plan_md field`);
  }
  if (typeof master.plan_md !== 'string') {
    throw new Error(`${label}: plan_md must be a string, got ${typeof master.plan_md}`);
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
      `${label}: plan_md does not match disk plan.md (len ${planMd.length} vs ${diskContent.length})`,
    );
  }
}

export function readPlanMdFromDisk(tasksDir, masterId) {
  const planPath = path.join(tasksDir, masterId, 'plan.md');
  try {
    return fs.readFileSync(planPath, 'utf8');
  } catch (err) {
    if (err && typeof err === 'object' && 'code' in err && err.code === 'ENOENT') {
      return '';
    }
    throw new Error(`failed to read plan.md for ${masterId}: ${err}`);
  }
}

export function writePlanMdToDisk(tasksDir, masterId, content) {
  const taskDir = path.join(tasksDir, masterId);
  fs.mkdirSync(taskDir, { recursive: true });
  fs.writeFileSync(path.join(taskDir, 'plan.md'), content, 'utf8');
}

function runSelfTest() {
  assertMasterPlanMdFields({ master_task_id: 'x', plan_md: '', migration_error: false }, 'ok');
  try {
    assertMasterPlanMdFields({ master_task_id: 'x' }, 'bad');
    throw new Error('expected missing plan_md to throw');
  } catch (err) {
    if (!String(err).includes('missing top-level plan_md')) {
      throw err;
    }
  }
  assertPlanMdMatchesDisk('hello', 'hello', 'match');
  console.log('plan-task-mcp-e2e self-test PASSED');
}

if (process.argv.includes('--self-test')) {
  runSelfTest();
  process.exit(0);
}

const mcpPort = Number(process.env.MCP_PORT);
if (!mcpPort) {
  console.error('plan-task-mcp-e2e: MCP_PORT is required');
  process.exit(1);
}

const tasksDir = process.env.E2E_PLAN_TASKS_TASKS_DIR?.trim() || '';

const PLAN_TOOLS = [
  'create_plan_task',
  'list_plan_tasks',
  'get_plan_task',
  'delete_plan_task',
  'add_plan_sub',
  'delete_plan_sub',
  'complete_plan',
  'link_plan_archive',
];

const transport = new StreamableHTTPClientTransport(
  new URL(`http://127.0.0.1:${mcpPort}/mcp`),
);
const client = new Client({ name: 'plan-task-mcp-e2e', version: '0.3.0' });

try {
  await client.connect(transport);
} catch (err) {
  console.error(`plan-task-mcp-e2e: MCP connect failed (is sidecar running on MCP_PORT=${mcpPort}?): ${err}`);
  process.exit(1);
}

const tools = await client.listTools();
const names = tools.tools.map((t) => t.name);
for (const tool of PLAN_TOOLS) {
  if (!names.includes(tool)) {
    throw new Error(`${tool} missing from tools: ${names.join(', ')}`);
  }
}

function toolText(result) {
  return result.content?.[0]?.text || '';
}

function parseJson(text) {
  return JSON.parse(text);
}

async function callPlanTool(name, args = {}) {
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
  name: 'create_plan_task',
  arguments: { title: 'MCP E2E', plan_md: '## E2E plan\n\nBody' },
});
const createText = toolText(createResult);
if (createResult.isError) {
  throw new Error(
    `create_plan_task failed (Workbench HTTP may be down): ${createText}`,
  );
}
const created = parseJson(createText);
const masterId = created.master_task_id;
if (!masterId) {
  throw new Error(`unexpected create_plan_task response: ${createText}`);
}

const addA = await callPlanTool('add_plan_sub', {
  master_task_id: masterId,
  title: 'Sub A',
});
const addB = await callPlanTool('add_plan_sub', {
  master_task_id: masterId,
  title: 'Sub B',
});
const subA = addA.task.sub_tasks.find((s) => s.title === 'Sub A')?.sub_task_id;
const subB = addB.task.sub_tasks.find((s) => s.title === 'Sub B')?.sub_task_id;
if (!subA || !subB) {
  throw new Error(`unexpected add_plan_sub response after create: ${JSON.stringify({ addA, addB })}`);
}

const listResult = await client.callTool({ name: 'list_plan_tasks', arguments: {} });
const listText = toolText(listResult);
if (listResult.isError) {
  throw new Error(`list_plan_tasks failed (Workbench HTTP may be down): ${listText}`);
}
const listBody = parseJson(listText);
if (!Array.isArray(listBody)) {
  throw new Error(`list_plan_tasks expected JSON array: ${listText}`);
}
const listMaster = listBody.find((t) => t.master_task_id === masterId);
if (!listMaster) {
  throw new Error(`list_plan_tasks missing created master ${masterId}: ${listText}`);
}
assertMasterPlanMdFields(listMaster, 'list_plan_tasks');
const expectedInitialPlanMd = '## E2E plan\n\nBody';
const diskBefore = tasksDir ? readPlanMdFromDisk(tasksDir, masterId) : expectedInitialPlanMd;
assertPlanMdMatchesDisk(listMaster.plan_md, diskBefore, 'list_plan_tasks');
if (listMaster.plan_md !== expectedInitialPlanMd) {
  throw new Error(
    `list_plan_tasks plan_md should match create plan_md, got: ${JSON.stringify(listMaster.plan_md)}`,
  );
}

const getResult = await client.callTool({
  name: 'get_plan_task',
  arguments: { id: masterId },
});
const getText = toolText(getResult);
if (getResult.isError) {
  throw new Error(`get_plan_task failed: ${getText}`);
}
const getBody = parseJson(getText);
assertMasterPlanMdFields(getBody, 'get_plan_task');
if (getBody.plan_md !== listMaster.plan_md) {
  throw new Error('get_plan_task plan_md must match list_plan_tasks for same id');
}
assertPlanMdMatchesDisk(getBody.plan_md, diskBefore, 'get_plan_task');

if (tasksDir) {
  const fixtureContent = '# MCP E2E plan.md\n\nRound-trip fixture paragraph.\n';
  writePlanMdToDisk(tasksDir, masterId, fixtureContent);
  const listRow = findMaster(
    await callPlanTool('list_plan_tasks'),
    masterId,
    'list_plan_tasks after disk write',
  );
  assertPlanMdMatchesDisk(listRow.plan_md, fixtureContent, 'list_plan_tasks after disk write');
  const getAfterWrite = await callPlanTool('get_plan_task', { id: masterId });
  assertPlanMdMatchesDisk(getAfterWrite.plan_md, fixtureContent, 'get_plan_task after disk write');

  writePlanMdToDisk(tasksDir, masterId, '');
  const listEmptyRow = findMaster(
    await callPlanTool('list_plan_tasks'),
    masterId,
    'list_plan_tasks empty plan.md',
  );
  assertPlanMdMatchesDisk(listEmptyRow.plan_md, '', 'list_plan_tasks empty plan.md');
}

const unknownGet = await client.callTool({
  name: 'get_plan_task',
  arguments: { id: '00000000000000000000000000000000' },
});
if (!unknownGet.isError) {
  throw new Error('expected get_plan_task error for unknown id');
}

const addSubResult = await client.callTool({
  name: 'add_plan_sub',
  arguments: { master_task_id: masterId, title: 'Sub C' },
});
const addSubText = toolText(addSubResult);
if (addSubResult.isError || !addSubText.includes('Sub C')) {
  throw new Error(`add_plan_sub failed: ${addSubText}`);
}

const completeResult = await client.callTool({
  name: 'complete_plan',
  arguments: { master_task_id: masterId, sub_task_id: subA },
});
const completeText = toolText(completeResult);
if (completeResult.isError || !completeText.includes('complete')) {
  throw new Error(`complete_plan failed: ${completeText}`);
}

const archiveId = '138700959e5ddb1260c69e9e18169ac4';
const linkResult = await client.callTool({
  name: 'link_plan_archive',
  arguments: { master_task_id: masterId, sub_task_id: subA, archive_id: archiveId },
});
const linkText = toolText(linkResult);
if (linkResult.isError || !linkText.includes(archiveId)) {
  throw new Error(`link_plan_archive failed: ${linkText}`);
}

const deleteSubResult = await client.callTool({
  name: 'delete_plan_sub',
  arguments: { master_task_id: masterId, sub_task_id: subB },
});
if (deleteSubResult.isError) {
  throw new Error(`delete_plan_sub failed: ${toolText(deleteSubResult)}`);
}

const deleteMasterResult = await client.callTool({
  name: 'delete_plan_task',
  arguments: { master_task_id: masterId },
});
const deleteMasterText = toolText(deleteMasterResult);
if (deleteMasterResult.isError || !deleteMasterText.includes('"ok":true')) {
  throw new Error(`delete_plan_task failed: ${deleteMasterText}`);
}

const invalid = await client.callTool({
  name: 'create_plan_task',
  arguments: {
    title:
      'one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twentyone',
  },
});
if (!invalid.isError) {
  throw new Error('expected validation error for title over 20 words');
}

await client.close();
console.log('plan-task-mcp-e2e PASSED');
