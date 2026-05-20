/**
 * P2 write API: HTTP POST path ↔ Tauri command (tauriDriver postJson).
 * Command names align with tech-doc naming contract.
 */

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
      common_path: body.common_path,
      content: body.content,
    }),
  },
  '/api/update-comments': {
    cmd: 'update_comments',
    args: (body) => ({
      common_path: body.common_path,
      layer: body.layer,
      comment: body.comment,
      ts: body.ts,
    }),
  },
  '/api/reorder-comments': {
    cmd: 'reorder_comments',
    args: (body) => ({
      common_path: body.common_path,
      layer: body.layer,
      ids: body.ids,
    }),
  },
  '/api/update-highlights': {
    cmd: 'update_highlights',
    args: (body) => ({
      common_path: body.common_path,
      layer: body.layer,
      highlight: body.highlight,
      ts: body.ts,
    }),
  },
  '/api/update-links': {
    cmd: 'update_links',
    args: (body) => ({
      common_path: body.common_path,
      links: body.links,
    }),
  },
  '/api/set-done': {
    cmd: 'set_done',
    args: (body) => ({
      common_path: body.common_path,
      done: body.done,
    }),
  },
  '/api/set-importance': {
    cmd: 'set_importance',
    args: (body) => ({
      common_path: body.common_path,
      importance: body.importance,
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
