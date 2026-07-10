/**
 * P1 read API: HTTP path (fetchDriver) ↔ Tauri command (tauriDriver).
 * Query keys match `api.js` HTTP paths.
 */

/** @type {Record<string, { cmd: string, args?: (url: URL) => Record<string, unknown> }>} */
export const READ_API_INVOKE_MAP = {
  '/api/corpus-index': {
    cmd: 'get_corpus_index',
  },
  '/api/corpus-file': {
    cmd: 'get_corpus_file',
    args: (url) => ({
      layer: url.searchParams.get('layer') ?? '',
      path: url.searchParams.get('path') ?? '',
    }),
  },
  '/api/corpus-asset': {
    cmd: 'get_corpus_asset',
    args: (url) => ({
      layer: url.searchParams.get('layer') ?? '',
      base: url.searchParams.get('base') ?? '',
      href: url.searchParams.get('href') ?? '',
    }),
  },
  '/api/topics': { cmd: 'get_topics' },
  '/api/search-knowledge': {
    cmd: 'search_knowledge',
    args: (url) => ({
      q: url.searchParams.get('q') ?? '',
      limit: url.searchParams.has('limit')
        ? Number(url.searchParams.get('limit'))
        : undefined,
    }),
  },
  '/api/search-workbench': {
    cmd: 'search_workbench',
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
  '/api/infer-github-user-url': {
    cmd: 'infer_github_user_url',
    args: (url) => ({ path: url.searchParams.get('path') ?? '' }),
  },
  '/api/check-workbench-root': {
    cmd: 'check_workbench_knowledge_root',
    args: (url) => ({ path: url.searchParams.get('path') ?? '' }),
  },
  '/api/status': { cmd: 'get_status' },
  '/api/read-later': { cmd: 'get_read_later' },
  '/api/plan-tasks': { cmd: 'get_plan_tasks' },
  '/api/kb/read': {
    cmd: 'kb_read',
    args: (url) => ({
      repo: url.searchParams.get('repo') ?? '',
      path: url.searchParams.get('path') ?? '',
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
      const args = { repo: url.searchParams.get('repo') ?? '' };
      const hidePattern = url.searchParams.get('hide_pattern');
      if (hidePattern != null && hidePattern !== '') {
        args.hide_pattern = hidePattern;
      }
      const categoryId = url.searchParams.get('category_id');
      if (categoryId != null && categoryId !== '') {
        args.category_id = categoryId;
      }
      return args;
    },
  },
  '/api/kb/annotation': {
    cmd: 'kb_annotation',
    args: (url) => ({
      repo: url.searchParams.get('repo') ?? '',
      path: url.searchParams.get('path') ?? '',
    }),
  },
  '/api/kb/status': {
    cmd: 'kb_status',
    args: (url) => ({ repo: url.searchParams.get('repo') ?? '' }),
  },
  '/api/kb/diff-status': {
    cmd: 'get_kb_diff_status',
  },
  '/api/repo-dirs': {
    cmd: 'get_repo_dirs',
    args: (url) => ({ repo: url.searchParams.get('repo') ?? '' }),
  },
  '/api/check-file': {
    cmd: 'check_file',
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
};

/**
 * Strip volatile fields before deep-equal contract checks (V-1).
 * @param {unknown} value
 */
export function normalizeForContract(value) {
  if (value === null || typeof value !== 'object') return value;
  if (Array.isArray(value)) {
    return value.map(normalizeForContract);
  }
  const out = {};
  for (const [k, v] of Object.entries(value)) {
    if (k === 'cached_at' || k === '_') continue;
    out[k] = normalizeForContract(v);
  }
  return out;
}

/**
 * @param {string} pathAndQuery e.g. `/api/topics?_=123`
 */
export function resolveInvokeFromPath(pathAndQuery) {
  const url = new URL(pathAndQuery, 'http://local');
  const entry = READ_API_INVOKE_MAP[url.pathname];
  if (!entry) return null;
  return {
    cmd: entry.cmd,
    args: entry.args ? entry.args(url) : {},
  };
}
