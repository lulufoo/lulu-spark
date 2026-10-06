/**
 * P4 sync API: HTTP POST path ↔ Tauri command (tauriDriver postJson).
 * Rust sync commands take a single `payload: Value` argument.
 */

type Body = Record<string, unknown>;

function syncPayload(body: Body) {
  return { payload: body ?? {} };
}

type SyncInvokeEntry = {
  cmd: string;
  args: (body: Body) => Body;
};

export const SYNC_API_INVOKE_MAP: Record<string, SyncInvokeEntry> = {
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
  '/api/draft': {
    cmd: 'save_comment_draft',
    args: (body) =>
      syncPayload({
        common_path: body.common_path,
        content: body.content,
      }),
  },
};

/**
 * @param path e.g. `/api/delete`
 */
export function resolveSyncInvoke(
  path: string,
  body?: Body | null,
): { cmd: string; args: Body } | null {
  const pathname = path.startsWith('/') ? path : `/${path}`;
  const entry = SYNC_API_INVOKE_MAP[pathname];
  if (!entry) return null;
  return {
    cmd: entry.cmd,
    args: entry.args(body ?? {}),
  };
}
