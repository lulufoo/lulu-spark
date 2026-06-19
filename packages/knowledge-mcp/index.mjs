#!/usr/bin/env node
/**
 * Thin MCP sidecar — proxies Workbench HTTP only (no corpus fs reads).
 */
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { createMcpExpressApp } from '@modelcontextprotocol/sdk/server/express.js';
import * as z from 'zod';

const WORKBENCH_HTTP_URL = process.env.WORKBENCH_HTTP_URL;
if (!WORKBENCH_HTTP_URL) {
  console.error('[knowledge-mcp] WORKBENCH_HTTP_URL is required');
  process.exit(1);
}

const MCP_PORT = Number(process.env.MCP_PORT || 9876);

async function proxyGet(pathAndQuery) {
  const url = `${WORKBENCH_HTTP_URL}${pathAndQuery}`;
  const res = await fetch(url);
  const text = await res.text();
  if (!res.ok) {
    return { ok: false, status: res.status, text };
  }
  return { ok: true, text };
}

function toolError(status, text) {
  return {
    content: [{ type: 'text', text: `HTTP ${status}: ${text}` }],
    isError: true,
  };
}

function buildServer() {
  const server = new McpServer(
    { name: 'workbench-knowledge-mcp', version: '0.1.0' },
    { capabilities: {} },
  );

  server.registerTool(
    'get_corpus_index',
    {
      description: 'Proxy GET /api/corpus-index from Workbench read API',
      inputSchema: {},
    },
    async () => {
      const result = await proxyGet('/api/corpus-index');
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'get_corpus_file',
    {
      description: 'Proxy GET /api/corpus-file?layer=digest&path=...',
      inputSchema: {
        path: z.string().describe('digest relative path'),
      },
    },
    async ({ path }) => {
      const q = new URLSearchParams({ layer: 'digest', path });
      const result = await proxyGet(`/api/corpus-file?${q}`);
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  return server;
}

const app = createMcpExpressApp({ host: '127.0.0.1' });

app.post('/mcp', async (req, res) => {
  const server = buildServer();
  try {
    const transport = new StreamableHTTPServerTransport({
      sessionIdGenerator: undefined,
    });
    await server.connect(transport);
    await transport.handleRequest(req, res, req.body);
    res.on('close', () => {
      transport.close();
      server.close();
    });
  } catch (error) {
    console.error('[knowledge-mcp] MCP error:', error);
    if (!res.headersSent) {
      res.status(500).json({
        jsonrpc: '2.0',
        error: { code: -32603, message: 'Internal server error' },
        id: null,
      });
    }
  }
});

app.get('/health', (_req, res) => {
  res.json({ ok: true, mcp: `http://127.0.0.1:${MCP_PORT}/mcp` });
});

app.listen(MCP_PORT, '127.0.0.1', () => {
  console.error(`[knowledge-mcp] MCP Streamable HTTP on http://127.0.0.1:${MCP_PORT}/mcp`);
  console.error(`[knowledge-mcp] proxy → ${WORKBENCH_HTTP_URL}`);
});

process.on('SIGTERM', () => process.exit(0));
