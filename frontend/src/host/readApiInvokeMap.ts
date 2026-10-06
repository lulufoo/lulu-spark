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
  '/api/notes-index': {
    cmd: 'get_notes_index',
  },
  '/api/notes-file': {
    cmd: 'get_notes_file',
    args: (url) => ({
      layer: url.searchParams.get('layer') ?? '',
      path: url.searchParams.get('path') ?? '',
    }),
  },
  '/api/notes-asset': {
    cmd: 'get_notes_asset',
    args: (url) => ({
      layer: url.searchParams.get('layer') ?? '',
      base: url.searchParams.get('base') ?? '',
      href: url.searchParams.get('href') ?? '',
    }),
  },
  '/api/topics': { cmd: 'get_topics' },
  '/api/notes-categories': { cmd: 'list_notes_categories' },
  '/api/search-knowledge': {
    cmd: 'search_knowledge',
    args: (url) => ({
      q: url.searchParams.get('q') ?? '',
      limit: url.searchParams.has('limit')
        ? Number(url.searchParams.get('limit'))
        : undefined,
    }),
  },
  '/api/search-spark': {
    cmd: 'search_spark',
    args: (url) => ({
      q: url.searchParams.get('q') ?? '',
      limit: url.searchParams.has('limit')
        ? Number(url.searchParams.get('limit'))
        : undefined,
    }),
  },
  '/api/annotations': { cmd: 'get_annotations' },
  '/api/tags/registry': { cmd: 'get_tags_registry' },
  '/api/annotation': {
    cmd: 'get_annotation',
    args: (url) => ({ path: url.searchParams.get('path') ?? '' }),
  },
  '/api/draft': {
    cmd: 'get_draft',
    args: (url) => ({ path: url.searchParams.get('path') ?? '' }),
  },
  '/api/note-draft': {
    cmd: 'get_note_draft',
    args: (url) => ({ tempId: url.searchParams.get('temp_id') ?? '' }),
  },
  '/api/config': { cmd: 'get_config' },
  '/api/status': { cmd: 'get_status' },
  '/api/read-later': { cmd: 'get_read_later' },
  '/api/doc-highlights': {
    cmd: 'get_doc_highlights',
    args: (url) => ({
      key: url.searchParams.get('key') ?? '',
    }),
  },
  '/api/kb/read': {
    cmd: 'kb_read',
    args: (url) => ({
      repo: url.searchParams.get('repo') ?? '',
      path: url.searchParams.get('path') ?? '',
    }),
  },
  '/api/kb/asset': {
    cmd: 'get_kb_asset',
    args: (url) => ({
      repo: url.searchParams.get('repo') ?? '',
      base: url.searchParams.get('base') ?? '',
      href: url.searchParams.get('href') ?? '',
    }),
  },
  '/api/kb/list': {
    cmd: 'kb_list',
    args: (url) => ({
      repo: url.searchParams.get('repo') ?? '',
      path: url.searchParams.get('path') ?? '',
      mode: url.searchParams.get('mode') ?? 'flat',
    }),
  },
  '/api/kb/doc-count': {
    cmd: 'kb_doc_count',
    args: (url) => {
      const args: InvokeArgs = { repo: url.searchParams.get('repo') ?? '' };
      const categoryId = url.searchParams.get('category_id');
      if (categoryId != null && categoryId !== '') {
        args.category_id = categoryId;
      }
      return args;
    },
  },
  '/api/kb/hide-patterns': { cmd: 'get_kb_hide_patterns' },
  '/api/kb/viewer-state': { cmd: 'get_kb_viewer_state' },
  '/api/kb/annotation': {
    cmd: 'kb_annotation',
    args: (url) => ({
      repo: url.searchParams.get('repo') ?? '',
      path: url.searchParams.get('path') ?? '',
    }),
  },
  '/api/fetch-title': {
    cmd: 'fetch_link_title',
    args: (url) => ({ url: url.searchParams.get('url') ?? '' }),
  },
  '/api/sediment-kb/categories': {
    cmd: 'get_sediment_kb_categories',
  },
  '/api/sediment-kb/repos': {
    cmd: 'get_sediment_kb_repos',
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
 * @param pathAndQuery e.g. `/api/topics?_=123`
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
