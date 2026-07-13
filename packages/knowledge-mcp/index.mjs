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

/** Count title units: each CJK ideograph = 1; each whitespace-delimited word token = 1. */
export function titleUnitCount(title) {
  let units = 0;
  let inWord = false;
  for (const c of title) {
    const cp = c.codePointAt(0);
    const isCjk =
      (cp >= 0x4e00 && cp <= 0x9fff) ||
      (cp >= 0x3400 && cp <= 0x4dbf) ||
      (cp >= 0xf900 && cp <= 0xfaff) ||
      (cp >= 0x20000 && cp <= 0x2a6df) ||
      (cp >= 0x2a700 && cp <= 0x2b73f) ||
      (cp >= 0x2b740 && cp <= 0x2b81f) ||
      (cp >= 0x2b820 && cp <= 0x2ceaf) ||
      (cp >= 0x2ceb0 && cp <= 0x2ebef) ||
      (cp >= 0x30000 && cp <= 0x3134f);
    if (isCjk) {
      if (inWord) {
        units += 1;
        inWord = false;
      }
      units += 1;
    } else if (/\s/u.test(c)) {
      if (inWord) {
        units += 1;
        inWord = false;
      }
    } else if (/[0-9A-Za-z\u00C0-\u024F\u1E00-\u1EFF]/u.test(c)) {
      inWord = true;
    } else if (inWord) {
      units += 1;
      inWord = false;
    }
  }
  if (inWord) {
    units += 1;
  }
  return units;
}

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

  server.registerTool(
    'create_plan_task',
    {
      description:
        'Create a plan task master with title and optional plan body (plan_md). Title max 20 Chinese characters or English words. Creates empty sub_tasks; use add_plan_sub for subs. Proxy POST /api/plan-task-create',
      inputSchema: {
        title: z
          .string()
          .trim()
          .min(1)
          .refine((t) => titleUnitCount(t) <= 20, {
            message: 'Title too long (max 20 Chinese characters or English words)',
          })
          .describe('Master task title (max 20 Chinese characters or English words)'),
        plan_md: z
          .string()
          .optional()
          .describe('Plan body markdown; omit or empty → empty plan.md'),
      },
    },
    async ({ title, plan_md }) => {
      const body = { title };
      if (plan_md != null) {
        body.plan_md = plan_md;
      }
      const result = await proxyPost('/api/plan-task-create', body);
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'list_plan_tasks',
    {
      description: 'List all plan task masters. Proxy GET /api/plan-tasks',
      inputSchema: {},
    },
    async () => {
      const result = await proxyGet('/api/plan-tasks');
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'get_plan_task',
    {
      description: 'Get a plan task master by id. Proxy GET /api/plan-task?id=',
      inputSchema: {
        id: z.string().trim().min(1).describe('Master task id'),
      },
    },
    async ({ id }) => {
      const q = new URLSearchParams({ id });
      const result = await proxyGet(`/api/plan-task?${q}`);
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'delete_plan_task',
    {
      description: 'Delete a plan task master and its directory. Proxy POST /api/plan-task-delete',
      inputSchema: {
        master_task_id: z.string().trim().min(1).describe('Master task id'),
      },
    },
    async ({ master_task_id }) => {
      const result = await proxyPost('/api/plan-task-delete', { master_task_id });
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'add_plan_sub',
    {
      description: 'Add a sub task to a plan master. Proxy POST /api/plan-task-add-sub',
      inputSchema: {
        master_task_id: z.string().trim().min(1).describe('Master task id'),
        title: z.string().trim().min(1).describe('Sub task title'),
      },
    },
    async ({ master_task_id, title }) => {
      const result = await proxyPost('/api/plan-task-add-sub', { master_task_id, title });
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'delete_plan_sub',
    {
      description: 'Delete a sub task (not the last one). Proxy POST /api/plan-task-delete-sub',
      inputSchema: {
        master_task_id: z.string().trim().min(1).describe('Master task id'),
        sub_task_id: z.string().trim().min(1).describe('Sub task id'),
      },
    },
    async ({ master_task_id, sub_task_id }) => {
      const result = await proxyPost('/api/plan-task-delete-sub', { master_task_id, sub_task_id });
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'complete_plan_sub',
    {
      description: 'Mark a sub task complete. Proxy POST /api/plan-task-complete-sub',
      inputSchema: {
        master_task_id: z.string().trim().min(1).describe('Master task id'),
        sub_task_id: z.string().trim().min(1).describe('Sub task id'),
      },
    },
    async ({ master_task_id, sub_task_id }) => {
      const result = await proxyPost('/api/plan-task-complete-sub', { master_task_id, sub_task_id });
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'link_plan_archive',
    {
      description:
        'Link an archive entry id to a completed sub task. Proxy POST /api/plan-task-link-archive',
      inputSchema: {
        master_task_id: z.string().trim().min(1).describe('Master task id'),
        sub_task_id: z.string().trim().min(1).describe('Sub task id'),
        archive_id: z.string().trim().min(1).describe('Archive entry id (32-char hex)'),
      },
    },
    async ({ master_task_id, sub_task_id, archive_id }) => {
      const result = await proxyPost('/api/plan-task-link-archive', {
        master_task_id,
        sub_task_id,
        archive_id,
      });
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
