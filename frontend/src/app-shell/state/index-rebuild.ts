import { createModuleStore } from '../../shared/module-store.ts';

export type IndexRebuildStatus = 'idle' | 'running' | 'done' | 'error';

export type IndexRebuildState = {
  status: IndexRebuildStatus;
  log: string;
};

/** Single header control; mirrors backend `get_reindex_all_status`. */
export const indexRebuildStore = createModuleStore<IndexRebuildState>({
  status: 'idle',
  log: '',
});
