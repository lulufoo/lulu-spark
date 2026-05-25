/**
 * P4 sync API: HTTP POST path ↔ Tauri command (tauriDriver postJson).
 * Rust sync commands take a single `payload: Value` argument.
 */

/** @param {Record<string, unknown>} body */
function syncPayload(body) {
  return { payload: body ?? {} };
}

/** @type {Record<string, { cmd: string, args: (body: Record<string, unknown>) => Record<string, unknown> }>} */
export const SYNC_API_INVOKE_MAP = {
  '/api/commit': {
    cmd: 'corpus_git_commit',
    args: (body) =>
      syncPayload({
        message: body.message,
        files: body.files,
      }),
  },
  '/api/pull': {
    cmd: 'corpus_git_pull',
    args: () => syncPayload({}),
  },
  '/api/delete': {
    cmd: 'delete_entry',
    args: (body) => syncPayload({ id: body.id }),
  },
  '/api/move-project': {
    cmd: 'move_entry_project',
    args: (body) =>
      syncPayload({
        id: body.id,
        new_project: body.new_project,
      }),
  },
  '/api/gh-move': {
    cmd: 'gh_move_assets',
    args: (body) =>
      syncPayload({
        src_url: body.src_url,
        dst_dir_url: body.dst_dir_url,
      }),
  },
  '/api/settle': {
    cmd: 'settle_entry',
    args: (body) =>
      syncPayload({
        common_path: body.common_path,
        comment_id: body.comment_id,
        layer: body.layer,
        doc_theme: body.doc_theme,
        slug: body.slug,
        content: body.content,
      }),
  },
  '/api/draft': {
    cmd: 'save_comment_draft',
    args: (body) =>
      syncPayload({
        common_path: body.common_path,
        content: body.content,
      }),
  },
  '/api/kb/commit': {
    cmd: 'kb_git_commit',
    args: (body) =>
      syncPayload({
        repo: body.repo,
        message: body.message,
      }),
  },
  '/api/kb/revert': {
    cmd: 'kb_git_revert',
    args: (body) =>
      syncPayload({
        repo: body.repo,
        path: body.path,
        type: body.type,
      }),
  },
  '/api/corpus-revert': {
    cmd: 'corpus_git_revert',
    args: (body) => syncPayload({ path: body.path, type: body.type }),
  },
  '/api/open-iterm': {
    cmd: 'open_kb_in_iterm',
    args: (body) => syncPayload({ repo: body.repo }),
  },
};

/**
 * @param {string} path e.g. `/api/commit`
 * @param {Record<string, unknown>} body
 */
export function resolveSyncInvoke(path, body) {
  const pathname = path.startsWith('/') ? path : `/${path}`;
  const entry = SYNC_API_INVOKE_MAP[pathname];
  if (!entry) return null;
  return {
    cmd: entry.cmd,
    args: entry.args(body ?? {}),
  };
}
