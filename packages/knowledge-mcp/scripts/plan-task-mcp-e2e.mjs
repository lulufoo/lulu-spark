#!/usr/bin/env node
/**
 * E2E: plan MCP tools against live sidecar + Workbench HTTP.
 * Env: MCP_PORT (required)
 */
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js';

const mcpPort = Number(process.env.MCP_PORT);
if (!mcpPort) {
  console.error('MCP_PORT is required');
  process.exit(1);
}

const PLAN_TOOLS = [
  'create_plan_task',
  'list_plan_tasks',
  'get_plan_task',
  'delete_plan_task',
  'add_plan_sub',
  'delete_plan_sub',
  'complete_plan_sub',
  'link_plan_archive',
];

const transport = new StreamableHTTPClientTransport(
  new URL(`http://127.0.0.1:${mcpPort}/mcp`),
);
const client = new Client({ name: 'plan-task-mcp-e2e', version: '0.2.0' });
await client.connect(transport);

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

const createResult = await client.callTool({
  name: 'create_plan_task',
  arguments: { title: 'MCP E2E', sub_titles: ['Sub A', 'Sub B'] },
});
const createText = toolText(createResult);
if (createResult.isError) {
  throw new Error(`create_plan_task failed: ${createText}`);
}
const created = parseJson(createText);
const masterId = created.master_task_id;
const subA = created.task.sub_tasks.find((s) => s.title === 'Sub A')?.sub_task_id;
const subB = created.task.sub_tasks.find((s) => s.title === 'Sub B')?.sub_task_id;
if (!masterId || !subA || !subB) {
  throw new Error(`unexpected create_plan_task response: ${createText}`);
}

const listResult = await client.callTool({ name: 'list_plan_tasks', arguments: {} });
const listText = toolText(listResult);
if (listResult.isError || !listText.includes(masterId)) {
  throw new Error(`list_plan_tasks failed: ${listText}`);
}

const getResult = await client.callTool({
  name: 'get_plan_task',
  arguments: { id: masterId },
});
const getText = toolText(getResult);
if (getResult.isError || !getText.includes('Sub B')) {
  throw new Error(`get_plan_task failed: ${getText}`);
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
  name: 'complete_plan_sub',
  arguments: { master_task_id: masterId, sub_task_id: subA },
});
const completeText = toolText(completeResult);
if (completeResult.isError || !completeText.includes('complete')) {
  throw new Error(`complete_plan_sub failed: ${completeText}`);
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
  arguments: { title: 'Bad', sub_titles: ['  '] },
});
if (!invalid.isError) {
  throw new Error('expected validation error for blank sub_titles element');
}

await client.close();
console.log('plan-task-mcp-e2e PASSED');
