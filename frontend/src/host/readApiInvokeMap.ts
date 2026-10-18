/**
 * P1 read API: HTTP path (fetchDriver) ↔ Tauri command (tauriDriver).
 * Query keys match `api.ts` HTTP paths.
 */

type InvokeArgs = Record<string, unknown>;

type ReadInvokeEntry = {
  cmd: string;
  args?: (url: URL) => InvokeArgs;
};

export const READ_API_INVOKE_MAP: Record<string, ReadInvokeEntry> = {
  '/api/docs-index': {
    cmd: 'get_docs_index',
    args: (url) => ({ workspaceId: url.searchParams.get('workspace_id') || undefined }),
  },
  '/api/doc-file': {
    cmd: 'get_doc_file',
    args: (url) => ({
      layer: url.searchParams.get('layer') ?? '',
      id: url.searchParams.get('id') ?? '',
      lang: url.searchParams.get('lang') || undefined,
    }),
  },
  '/api/doc-asset': {
    cmd: 'get_doc_asset',
    args: (url) => ({
      layer: url.searchParams.get('layer') ?? '',
      id: url.searchParams.get('id') ?? '',
      href: url.searchParams.get('href') ?? '',
      lang: url.searchParams.get('lang') || undefined,
    }),
  },
  '/api/workspaces': { cmd: 'list_workspaces' },
  '/api/workspaces-disabled': { cmd: 'list_disabled_workspaces' },
  '/api/projects': {
    cmd: 'list_projects',
    args: (url) => ({ payload: { workspace_id: url.searchParams.get('workspace_id') || undefined } }),
  },
  '/api/hide-patterns': {
    cmd: 'list_hide_patterns',
    args: (url) => ({ payload: { workspace_id: url.searchParams.get('workspace_id') || undefined } }),
  },
  '/api/viewer-state': {
    cmd: 'get_viewer_state',
    args: (url) => ({ payload: { workspace_id: url.searchParams.get('workspace_id') || undefined } }),
  },
  '/api/search-spark': {
    cmd: 'search_spark',
    args: (url) => ({
      q: url.searchParams.get('q') ?? '',
      workspaceId: url.searchParams.get('workspace_id') || undefined,
      limit: url.searchParams.has('limit')
        ? Number(url.searchParams.get('limit'))
        : undefined,
    }),
  },
  '/api/annotations': { cmd: 'get_annotations' },
  '/api/tags/registry': { cmd: 'get_tags_registry' },
  '/api/annotation': {
    cmd: 'get_annotation',
    args: (url) => ({ id: url.searchParams.get('id') ?? '' }),
  },
  '/api/draft': {
    cmd: 'get_draft',
    args: (url) => ({ id: url.searchParams.get('id') ?? '' }),
  },
  '/api/doc-draft': {
    cmd: 'get_doc_draft',
    args: (url) => ({ tempId: url.searchParams.get('temp_id') ?? '' }),
  },
  '/api/config': { cmd: 'get_config' },
  '/api/read-later': { cmd: 'get_read_later' },
  '/api/doc-highlights': {
    cmd: 'get_doc_highlights',
    args: (url) => ({
      key: url.searchParams.get('key') ?? '',
    }),
  },
  '/api/fetch-title': {
    cmd: 'fetch_link_title',
    args: (url) => ({ url: url.searchParams.get('url') ?? '' }),
  },
  '/api/file': {
    cmd: 'read_abs_file',
    args: (url) => ({
      path: url.searchParams.get('path') ?? '',
    }),
  },
  '/api/message-channel-unread': {
    cmd: 'get_message_channel_unread',
    args: (url) => ({
      channel: url.searchParams.get('channel') ?? '',
    }),
  },
};

/**
 * Strip volatile fields before deep-equal contract checks (V-1).
 */
export function normalizeForContract(value: unknown): unknown {
  if (value === null || typeof value !== 'object') return value;
  if (Array.isArray(value)) {
    return value.map(normalizeForContract);
  }
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
    if (k === 'cached_at' || k === '_') continue;
    out[k] = normalizeForContract(v);
  }
  return out;
}

/**
 * @param pathAndQuery e.g. `/api/workspaces?_=123`
 */
export function resolveInvokeFromPath(
  pathAndQuery: string,
): { cmd: string; args: InvokeArgs } | null {
  const url = new URL(pathAndQuery, 'http://local');
  const entry = READ_API_INVOKE_MAP[url.pathname];
  if (!entry) return null;
  return {
    cmd: entry.cmd,
    args: entry.args ? entry.args(url) : {},
  };
}
