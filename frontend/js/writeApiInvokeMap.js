/**
 * P2 write API: HTTP POST path ↔ Tauri command (tauriDriver postJson).
 * Command names align with tech-doc naming contract.
 * HTTP bodies use snake_case; Tauri invoke args use camelCase (Tauri 2 IPC).
 */

/** @param {Record<string, unknown>} body */
function commonPathArg(body) {
  return { commonPath: body.common_path };
}

/** @type {Record<string, { cmd: string, args: (body: Record<string, unknown>) => Record<string, unknown> }>} */
export const WRITE_API_INVOKE_MAP = {
  '/api/config': {
    cmd: 'set_config',
    args: (body) => ({
      payload: body ?? {},
    }),
  },
  '/api/save': {
    cmd: 'save_entry',
    args: (body) => ({
      layer: body.layer,
      ...commonPathArg(body),
      content: body.content,
    }),
  },
  '/api/update-comments': {
    cmd: 'update_comments',
    args: (body) => ({
      ...commonPathArg(body),
      layer: body.layer,
      comment: body.comment,
      ts: body.ts,
    }),
  },
  '/api/reorder-comments': {
    cmd: 'reorder_comments',
    args: (body) => ({
      ...commonPathArg(body),
      layer: body.layer,
      ids: body.ids,
    }),
  },
  '/api/update-highlights': {
    cmd: 'update_highlights',
    args: (body) => ({
      ...commonPathArg(body),
      layer: body.layer,
      highlight: body.highlight,
      ts: body.ts,
    }),
  },
  '/api/update-links': {
    cmd: 'update_links',
    args: (body) => ({
      ...commonPathArg(body),
      links: body.links,
    }),
  },
  '/api/set-done': {
    cmd: 'set_done',
    args: (body) => ({
      ...commonPathArg(body),
      done: body.done,
    }),
  },
  '/api/set-importance': {
    cmd: 'set_importance',
    args: (body) => ({
      ...commonPathArg(body),
      importance: body.importance,
    }),
  },
  '/api/read-later': {
    cmd: 'create_read_later',
    args: (body) => ({
      url: body.url,
      ...(body.title != null ? { title: body.title } : {}),
    }),
  },
  '/api/kb/save': {
    cmd: 'kb_save',
    args: (body) => ({
      repo: body.repo,
      path: body.path,
      content: body.content,
    }),
  },
  '/api/kb/update-comments': {
    cmd: 'kb_update_comments',
    args: (body) => ({
      repo: body.repo,
      path: body.path,
      comment: body.comment,
      ts: body.ts,
    }),
  },
  '/api/kb/reorder-comments': {
    cmd: 'kb_reorder_comments',
    args: (body) => ({
      repo: body.repo,
      path: body.path,
      ids: body.ids,
    }),
  },
  '/api/kb/update-highlights': {
    cmd: 'kb_update_highlights',
    args: (body) => ({
      repo: body.repo,
      path: body.path,
      highlight: body.highlight,
      ts: body.ts,
    }),
  },
  '/api/kb/update-links': {
    cmd: 'kb_update_links',
    args: (body) => ({
      repo: body.repo,
      path: body.path,
      links: body.links,
    }),
  },
  '/api/tag/attach': {
    cmd: 'tag_attach',
    args: (body) => ({
      commonPath: body.common_path,
      ...(body.key != null && body.key !== '' ? { key: body.key } : {}),
      ...(body.value != null && body.value !== '' ? { value: body.value } : {}),
    }),
  },
  '/api/tag/detach': {
    cmd: 'tag_detach',
    args: (body) => ({
      commonPath: body.common_path,
      key: body.key,
    }),
  },
  '/api/tag/update-value': {
    cmd: 'tag_update_value',
    args: (body) => ({
      key: body.key,
      value: body.value,
    }),
  },
  '/api/sediment-kb/repos/add': {
    cmd: 'sediment_kb_add_repo',
    args: (body) => ({ payload: body ?? {} }),
  },
  '/api/sediment-kb/repos/remove': {
    cmd: 'sediment_kb_remove_repo',
    args: (body) => ({ payload: body ?? {} }),
  },
  '/api/sediment-kb/repos/update-category': {
    cmd: 'sediment_kb_update_repo_category',
    args: (body) => ({ payload: body ?? {} }),
  },
  '/api/sediment-kb/categories/add': {
    cmd: 'sediment_kb_add_category',
    args: (body) => ({ payload: body ?? {} }),
  },
  '/api/sediment-kb/categories/rename': {
    cmd: 'sediment_kb_rename_category',
    args: (body) => ({ payload: body ?? {} }),
  },
  '/api/sediment-kb/categories/remove': {
    cmd: 'sediment_kb_remove_category',
    args: (body) => ({ payload: body ?? {} }),
  },
};

/**
 * @param {string} path e.g. `/api/set-done`
 * @param {Record<string, unknown>} body
 */
export function resolveWriteInvoke(path, body) {
  const pathname = path.startsWith('/') ? path : `/${path}`;
  const entry = WRITE_API_INVOKE_MAP[pathname];
  if (!entry) return null;
  return {
    cmd: entry.cmd,
    args: entry.args(body ?? {}),
  };
}
