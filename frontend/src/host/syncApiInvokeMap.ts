/**
 * P4 sync API: HTTP POST path ↔ Tauri command (tauriDriver postJson).
 * Rust sync commands take a single `payload: Value` argument, except `sync_documents` and
 * `cancel_sync_documents`, which take none: the Rust side reads the session from the vault, and a
 * cancel has nothing to say but "stop".
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
        project_id: body.project_id,
        ...(body.target_workspace_id
          ? { target_workspace_id: body.target_workspace_id }
          : {}),
      }),
  },
  '/api/cloud-sync': {
    cmd: 'sync_documents',
    args: () => ({}),
  },
  '/api/cloud-sync-cancel': {
    cmd: 'cancel_sync_documents',
    args: () => ({}),
  },
  '/api/draft': {
    cmd: 'save_comment_draft',
    args: (body) =>
      syncPayload({
        id: body.id,
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
