/**
 * P3 reindex: direct Tauri invoke (not readDriver / readApiInvokeMap).
 */

type Payload = Record<string, unknown>;

type ReindexEntry = {
  cmd: string;
  args?: (payload?: Payload) => Payload;
};

export const REINDEX_INVOKE_MAP: Record<string, ReindexEntry> = {
  reindexKnowledge: { cmd: 'reindex_knowledge' },
  reindexWorkbench: { cmd: 'reindex_workbench' },
  reindexKbRepo: {
    cmd: 'reindex_kb_repo',
    args: (payload) => ({ repo: payload?.repo ?? '' }),
  },
  syncKnowledgeCorpus: { cmd: 'sync_knowledge_corpus' },
  getReindexStatus: { cmd: 'get_reindex_status' },
  getReindexWorkbenchStatus: { cmd: 'get_reindex_workbench_status' },
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
