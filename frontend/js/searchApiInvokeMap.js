/**
 * P3 reindex: direct Tauri invoke (not readDriver / readApiInvokeMap).
 */

/** @type {Record<string, { cmd: string, args?: (payload?: Record<string, unknown>) => Record<string, unknown> }>} */
export const REINDEX_INVOKE_MAP = {
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

/**
 * @param {'reindexKnowledge'|'reindexWorkbench'|'reindexKbRepo'|'syncKnowledgeCorpus'|'getReindexStatus'|'getReindexWorkbenchStatus'} key
 * @param {Record<string, unknown>} [payload]
 */
export function resolveReindexInvoke(key, payload) {
  const entry = REINDEX_INVOKE_MAP[key];
  if (!entry) return null;
  return {
    cmd: entry.cmd,
    args: entry.args ? entry.args(payload) : {},
  };
}
