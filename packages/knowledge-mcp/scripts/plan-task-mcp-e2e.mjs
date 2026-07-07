#!/usr/bin/env node
/**
 * E2E: create_plan_task MCP tool against live sidecar + Workbench HTTP.
 * Env: MCP_PORT (required)
 */
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js';

const mcpPort = Number(process.env.MCP_PORT);
if (!mcpPort) {
  console.error('MCP_PORT is required');
  process.exit(1);
}

const transport = new StreamableHTTPClientTransport(
  new URL(`http://127.0.0.1:${mcpPort}/mcp`),
);
const client = new Client({ name: 'plan-task-mcp-e2e', version: '0.1.0' });
await client.connect(transport);

const tools = await client.listTools();
const names = tools.tools.map((t) => t.name);
if (!names.includes('create_plan_task')) {
  throw new Error(`create_plan_task missing from tools: ${names.join(', ')}`);
}

const result = await client.callTool({
  name: 'create_plan_task',
  arguments: { title: 'MCP E2E', sub_titles: ['Explicit sub'] },
});
const text = result.content?.[0]?.text || '';
if (result.isError) {
  throw new Error(`create_plan_task failed: ${text}`);
}
if (!text.includes('"implicit":false') || !text.includes('Explicit sub')) {
  throw new Error(`unexpected create_plan_task response: ${text}`);
}

const invalid = await client.callTool({
  name: 'create_plan_task',
  arguments: { title: 'Bad', sub_titles: ['  '] },
});
if (!invalid.isError) {
  throw new Error('expected validation error for blank sub_titles element');
}

await client.close();
