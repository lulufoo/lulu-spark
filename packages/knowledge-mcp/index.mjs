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

async function proxyPost(path, body) {
  const url = `${WORKBENCH_HTTP_URL}${path}`;
  const res = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
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
    { name: 'workbench-knowledge-mcp', version: '0.3.0' },
    { capabilities: {} },
  );

  server.registerTool(
    'get_corpus_catalog',
    {
      description:
        'Slim digest catalog: latest entry per top-level topic (id, topic, created_at). Proxy GET /api/corpus-catalog?mode=latest_per_topic',
      inputSchema: {
        mode: z
          .literal('latest_per_topic')
          .describe('Catalog mode; only latest_per_topic is supported'),
      },
    },
    async ({ mode }) => {
      const q = new URLSearchParams({ mode });
      const result = await proxyGet(`/api/corpus-catalog?${q}`);
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'get_corpus_files',
    {
      description: 'Batch-read digest bodies by index entry id. Proxy POST /api/corpus-files',
      inputSchema: {
        ids: z
          .array(z.string())
          .min(1)
          .max(32)
          .describe('Index entry ids (32-char hex)'),
      },
    },
    async ({ ids }) => {
      const result = await proxyPost('/api/corpus-files', { ids });
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'archive_document',
    {
      description:
        'Archive a formatted document to raw/ and index.json. Returns entry id. Proxy POST /api/archive-document',
      inputSchema: {
        document: z.string().min(1).describe('Full Markdown document with header and body'),
        source_type: z
          .string()
          .optional()
          .describe('Index source_type; defaults to summary'),
        extra_documents: z
          .array(
            z.object({
              rel: z.string().min(1).describe('raw/<topic-path>/<ts>-<slug>-zh.md'),
              content: z.string().min(1).describe('Extra raw markdown body'),
            }),
          )
          .optional()
          .describe('Optional extra raw files (theme-line -zh.md)'),
        index_extra: z
          .object({
            translations: z
              .object({
                zh: z.string().min(1).describe('common_path for -zh.md translation'),
              })
              .optional(),
          })
          .optional()
          .describe('Optional index fields; only translations.zh supported'),
      },
    },
    async ({ document, source_type, extra_documents, index_extra }) => {
      const body = { document };
      if (source_type != null) {
        body.source_type = source_type;
      }
      if (extra_documents != null) {
        body.extra_documents = extra_documents;
      }
      if (index_extra != null) {
        body.index_extra = index_extra;
      }
      const result = await proxyPost('/api/archive-document', body);
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'archive_digest',
    {
      description:
        'Write digest markdown for an existing entry id and append digest layer. Proxy POST /api/archive-digest',
      inputSchema: {
        id: z.string().length(32).describe('Entry id from archive_document'),
        digest: z.string().min(1).describe('Full digest Markdown'),
        force: z
          .boolean()
          .optional()
          .describe('Overwrite existing digest; default false'),
      },
    },
    async ({ id, digest, force }) => {
      const body = { id, digest };
      if (force != null) {
        body.force = force;
      }
      const result = await proxyPost('/api/archive-digest', body);
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
