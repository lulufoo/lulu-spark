#!/usr/bin/env node
/**
 * Thin MCP sidecar — proxies Workbench HTTP only (no corpus fs reads).
 * Isolation via path scene_slot: /mcp/<scene_slot>; unknown slots hard-fail.
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

/**
 * Registered scene_slot → API surface seeds (permission SSOT).
 * - todo_task: App Binding key; todo tools only
 * - cursor_ide: external IDE channel; corpus/archive + todo tools
 * @type {Readonly<Record<string, { includeCorpus: boolean, includeTodo: boolean }>>}
 */
const SCENE_SLOT_API = Object.freeze({
  todo_task: Object.freeze({ includeCorpus: false, includeTodo: true }),
  cursor_ide: Object.freeze({ includeCorpus: true, includeTodo: true }),
});

const REGISTERED_SCENE_SLOTS = Object.freeze(Object.keys(SCENE_SLOT_API));

/**
 * Resolve registered scene_slot from path param. Unknown → null (caller hard-rejects).
 * @param {unknown} raw
 * @returns {string|null}
 */
export function resolveSceneSlot(raw) {
  if (raw == null) return null;
  const key = String(raw).trim();
  if (!key || !Object.prototype.hasOwnProperty.call(SCENE_SLOT_API, key)) return null;
  return key;
}

function unreachableProxyResult(err) {
  const message = err instanceof Error ? err.message : String(err);
  return { ok: false, status: 503, text: `Workbench HTTP unreachable: ${message}` };
}

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
  try {
    const res = await fetch(url);
    const text = await res.text();
    if (!res.ok) {
      return { ok: false, status: res.status, text };
    }
    return { ok: true, text };
  } catch (err) {
    return unreachableProxyResult(err);
  }
}

async function proxyPost(path, body) {
  const url = `${WORKBENCH_HTTP_URL}${path}`;
  try {
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
  } catch (err) {
    return unreachableProxyResult(err);
  }
}

function toolError(status, text) {
  return {
    content: [{ type: 'text', text: `HTTP ${status}: ${text}` }],
    isError: true,
  };
}

/** Thin proxy handler: forward tool args as JSON body via proxyPost. */
function proxyPostHandler(path) {
  return async (args) => {
    const result = await proxyPost(path, args);
    if (!result.ok) {
      return toolError(result.status, result.text);
    }
    return { content: [{ type: 'text', text: result.text }] };
  };
}

/**
 * Build MCP server for a registered scene_slot API surface.
 * @param {string} sceneSlot — must be a key of SCENE_SLOT_API
 */
export function buildServer(sceneSlot) {
  const api = SCENE_SLOT_API[sceneSlot];
  if (!api) {
    throw new Error(`unregistered scene_slot: ${sceneSlot}`);
  }
  const { includeCorpus, includeTodo } = api;

  const server = new McpServer(
    { name: 'workbench-knowledge-mcp', version: '0.3.0' },
    { capabilities: {} },
  );

  if (includeCorpus) {
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
        translations: z
          .array(
            z.object({
              lang: z
                .string()
                .regex(/^[a-z]{2}$/)
                .describe('ISO 639-1 language code'),
              content: z.string().min(1).describe('Translation markdown body'),
            }),
          )
          .optional()
          .describe('Optional translation bodies; paths derived by host'),
      },
    },
    async ({ document, source_type, translations }) => {
      const body = { document };
      if (source_type != null) {
        body.source_type = source_type;
      }
      if (translations != null) {
        body.translations = translations;
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
  }

  if (includeTodo) {
  server.registerTool(
    'create_todo_task',
    {
      description:
        'Create a todo task master with title and optional todo body (todo_md). Title max 20 Chinese characters or English words. Creates empty sub_tasks; use add_todo_sub for subs. Proxy POST /api/todo-task-create',
      inputSchema: {
        title: z
          .string()
          .trim()
          .min(1)
          .refine((t) => titleUnitCount(t) <= 20, {
            message: 'Title too long (max 20 Chinese characters or English words)',
          })
          .describe('Master task title (max 20 Chinese characters or English words)'),
        todo_md: z
          .string()
          .optional()
          .describe('Todo body markdown; omit or empty → empty todo.md'),
      },
    },
    async ({ title, todo_md }) => {
      const body = { title };
      if (todo_md != null) {
        body.todo_md = todo_md;
      }
      const result = await proxyPost('/api/todo-task-create', body);
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'update_todo_task',
    {
      description:
        'Update a todo master title and/or body (todo_md). Provide master_task_id and at least one of title or todo_md. Omit a field to leave it unchanged; todo_md "" clears the body. Proxy POST /api/todo-task-update',
      inputSchema: {
        master_task_id: z.string().trim().min(1).describe('Master task id'),
        title: z
          .string()
          .trim()
          .min(1)
          .refine((t) => titleUnitCount(t) <= 20, {
            message: 'Title too long (max 20 Chinese characters or English words)',
          })
          .optional()
          .describe('New master title; omit → unchanged'),
        todo_md: z
          .string()
          .optional()
          .describe('Todo body markdown; omit → unchanged; empty string → clear'),
      },
    },
    async ({ master_task_id, title, todo_md }) => {
      if (title == null && todo_md == null) {
        return toolError(400, JSON.stringify({ error: 'Missing title or todo_md' }));
      }
      const body = { master_task_id };
      if (title != null) {
        body.title = title;
      }
      if (todo_md != null) {
        body.todo_md = todo_md;
      }
      const result = await proxyPost('/api/todo-task-update', body);
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'list_todo_tasks',
    {
      description: 'List all todo task masters. Proxy GET /api/todo-tasks',
      inputSchema: {},
    },
    async () => {
      const result = await proxyGet('/api/todo-tasks');
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'get_todo_task',
    {
      description: 'Get a todo task master by id. Proxy GET /api/todo-task?id=',
      inputSchema: {
        id: z.string().trim().min(1).describe('Master task id'),
      },
    },
    async ({ id }) => {
      const q = new URLSearchParams({ id });
      const result = await proxyGet(`/api/todo-task?${q}`);
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'delete_todo_task',
    {
      description: 'Delete a todo task master and its directory. Proxy POST /api/todo-task-delete',
      inputSchema: {
        master_task_id: z.string().trim().min(1).describe('Master task id'),
      },
    },
    async ({ master_task_id }) => {
      const result = await proxyPost('/api/todo-task-delete', { master_task_id });
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'add_todo_sub',
    {
      description:
        'Add a sub task to a todo master with optional content. Title-only (master_task_id + title) remains valid. Proxy POST /api/todo-task-add-sub',
      inputSchema: {
        master_task_id: z.string().trim().min(1).describe('Master task id'),
        title: z.string().trim().min(1).describe('Sub task title'),
        content: z.string().optional().describe('Optional sub task content; omit → no content field'),
      },
    },
    async ({ master_task_id, title, content }) => {
      const body = { master_task_id, title };
      if (content != null) {
        body.content = content;
      }
      const result = await proxyPost('/api/todo-task-add-sub', body);
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'update_todo_sub',
    {
      description:
        'Update a sub task title and/or content. Provide master_task_id, sub_task_id, and at least one of title or content. Omit content to leave unchanged; content "" clears. When title is omitted, current title is resolved before proxy (HTTP requires non-empty title). Proxy POST /api/todo-task-update-sub',
      inputSchema: {
        master_task_id: z.string().trim().min(1).describe('Master task id'),
        sub_task_id: z.string().trim().min(1).describe('Sub task id'),
        title: z.string().trim().min(1).optional().describe('New sub title; omit → keep current (resolved before HTTP)'),
        content: z.string().optional().describe('Sub content; omit → unchanged; empty string → clear'),
      },
    },
    async ({ master_task_id, sub_task_id, title, content }) => {
      if (title == null && content == null) {
        return toolError(400, JSON.stringify({ error: 'Missing title or content' }));
      }
      let resolvedTitle = title;
      if (title == null) {
        const q = new URLSearchParams({ id: master_task_id });
        const got = await proxyGet(`/api/todo-task?${q}`);
        if (!got.ok) {
          return toolError(got.status, got.text);
        }
        let master;
        try {
          master = JSON.parse(got.text);
        } catch {
          return toolError(500, JSON.stringify({ error: 'Invalid todo-task response' }));
        }
        const subs = Array.isArray(master?.sub_tasks) ? master.sub_tasks : [];
        const sub = subs.find((s) => s && s.sub_task_id === sub_task_id);
        const current = typeof sub?.title === 'string' ? sub.title.trim() : '';
        if (!current) {
          return toolError(
            404,
            JSON.stringify({ error: 'Sub task not found or title unavailable' }),
          );
        }
        resolvedTitle = current;
      }
      const body = { master_task_id, sub_task_id };
      body.title = resolvedTitle;
      if (content != null) {
        body.content = content;
      }
      const result = await proxyPost('/api/todo-task-update-sub', body);
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'delete_todo_sub',
    {
      description: 'Delete a sub task (not the last one). Proxy POST /api/todo-task-delete-sub',
      inputSchema: {
        master_task_id: z.string().trim().min(1).describe('Master task id'),
        sub_task_id: z.string().trim().min(1).describe('Sub task id'),
      },
    },
    async ({ master_task_id, sub_task_id }) => {
      const result = await proxyPost('/api/todo-task-delete-sub', { master_task_id, sub_task_id });
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'complete_todo',
    {
      description:
        'Complete a todo task. With sub_task_id, marks that sub complete; without it, marks the master complete. Proxy POST /api/todo-task-complete',
      inputSchema: {
        master_task_id: z.string().trim().min(1).describe('Master task id'),
        sub_task_id: z
          .string()
          .trim()
          .min(1)
          .optional()
          .describe('Optional sub task id; omit to complete the master task'),
      },
    },
    async ({ master_task_id, sub_task_id }) => {
      const body = { master_task_id };
      if (sub_task_id) body.sub_task_id = sub_task_id;
      const result = await proxyPost('/api/todo-task-complete', body);
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );

  server.registerTool(
    'link_todo_archive',
    {
      description:
        'Link an archive entry id to a completed sub task. Proxy POST /api/todo-task-link-archive',
      inputSchema: {
        master_task_id: z.string().trim().min(1).describe('Master task id'),
        sub_task_id: z.string().trim().min(1).describe('Sub task id'),
        archive_id: z.string().trim().min(1).describe('Archive entry id (32-char hex)'),
      },
    },
    async ({ master_task_id, sub_task_id, archive_id }) => {
      const result = await proxyPost('/api/todo-task-link-archive', {
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

  server.registerTool(
    'add_todo_attachment',
    {
      description:
        'Add a markdown attachment to a todo master. Proxy POST /api/todo-task-add-attachment',
      inputSchema: {
        master_task_id: z.string().trim().min(1).describe('Master task id'),
        file_name: z.string().trim().min(1).describe('Attachment file name (must end with .md)'),
        content: z.string().describe('Markdown attachment content'),
      },
    },
    proxyPostHandler('/api/todo-task-add-attachment'),
  );

  server.registerTool(
    'list_todo_attachments',
    {
      description:
        'List attachments for a todo master. Proxy POST /api/todo-task-list-attachments',
      inputSchema: {
        master_task_id: z.string().trim().min(1).describe('Master task id'),
      },
    },
    proxyPostHandler('/api/todo-task-list-attachments'),
  );

  server.registerTool(
    'get_todo_attachment',
    {
      description:
        'Read one todo attachment body by file name. Proxy POST /api/todo-task-get-attachment',
      inputSchema: {
        master_task_id: z.string().trim().min(1).describe('Master task id'),
        file_name: z.string().trim().min(1).describe('Attachment file name'),
      },
    },
    proxyPostHandler('/api/todo-task-get-attachment'),
  );

  server.registerTool(
    'update_todo_attachment',
    {
      description:
        'Overwrite a todo attachment body. Proxy POST /api/todo-task-update-attachment',
      inputSchema: {
        master_task_id: z.string().trim().min(1).describe('Master task id'),
        file_name: z.string().trim().min(1).describe('Attachment file name'),
        content: z.string().describe('New markdown attachment content'),
      },
    },
    proxyPostHandler('/api/todo-task-update-attachment'),
  );
  }

  return server;
}

const app = createMcpExpressApp({ host: '127.0.0.1' });

// Bare /mcp (no scene_slot) is not a registered isolation surface — hard reject.
app.all('/mcp', (_req, res) => {
  res.status(404).json({
    error: 'scene_slot_required',
    message: `Use /mcp/<scene_slot> (registered: ${REGISTERED_SCENE_SLOTS.join(', ')})`,
  });
});

// Path scene_slot routing: /mcp/<scene_slot> → slot API config → tools/list.
// Unknown/unregistered slots hard-fail (never fall back to full tool set).
app.all('/mcp/:sceneSlot', async (req, res) => {
  const sceneSlot = resolveSceneSlot(req.params.sceneSlot);
  if (sceneSlot == null) {
    res.status(404).json({
      error: 'unknown_scene_slot',
      scene_slot: String(req.params.sceneSlot ?? ''),
      message: 'Unregistered scene_slot; connection rejected',
    });
    return;
  }

  const accept = String(req.headers.accept || '');
  if (req.method === 'GET' && !accept.includes('text/event-stream')) {
    res.json({
      ok: true,
      transport: 'streamable-http',
      scene_slot: sceneSlot,
      endpoint: `http://127.0.0.1:${MCP_PORT}/mcp/${sceneSlot}`,
      hint: `Use POST /mcp/${sceneSlot} for JSON-RPC and GET with Accept: text/event-stream for streams.`,
    });
    return;
  }

  const server = buildServer(sceneSlot);
  try {
    const transport = new StreamableHTTPServerTransport({
      sessionIdGenerator: undefined,
    });
    await server.connect(transport);
    await transport.handleRequest(req, res, req.body ?? undefined);
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
  res.json({
    ok: true,
    mcp: `http://127.0.0.1:${MCP_PORT}/mcp/<scene_slot>`,
    scene_slots: REGISTERED_SCENE_SLOTS,
  });
});

app.listen(MCP_PORT, '127.0.0.1', () => {
  console.error(
    `[knowledge-mcp] MCP Streamable HTTP on http://127.0.0.1:${MCP_PORT}/mcp/<scene_slot>`,
  );
  console.error(`[knowledge-mcp] registered slots: ${REGISTERED_SCENE_SLOTS.join(', ')}`);
  console.error(`[knowledge-mcp] proxy → ${WORKBENCH_HTTP_URL}`);
});

process.on('SIGTERM', () => process.exit(0));
