/**
 * P3 reindex: direct Tauri invoke (not readDriver / readApiInvokeMap).
 */

type Payload = Record<string, unknown>;

type ReindexEntry = {
  cmd: string;
  args?: (payload?: Payload) => Payload;
};

export const REINDEX_INVOKE_MAP: Record<string, ReindexEntry> = {
  reindexAll: { cmd: 'reindex_all' },
  getReindexAllStatus: { cmd: 'get_reindex_all_status' },
  reindexKbRepo: {
    cmd: 'reindex_kb_repo',
    args: (payload) => ({ repo: payload?.repo ?? '' }),
  },
  getReindexStatus: { cmd: 'get_reindex_status' },
};

export function resolveReindexInvoke(
  key: string,
  payload?: Payload,
): { cmd: string; args: Payload } | null {
  const entry = REINDEX_INVOKE_MAP[key];
  if (!entry) return null;
  return {
    cmd: entry.cmd,
    args: entry.args ? entry.args(payload) : {},
  };
}
