import { createModuleStore } from '../../../shared/module-store.ts';

export type SedimentKbRepo = {
  full_name: string;
  name: string;
  description: string;
  category_id: string;
  category_name: string;
  local_exists: boolean;
};

export type SedimentKbCategory = {
  id: string;
  name: string;
};

export type SedimentKbStatus = {
  full_name: string;
  name?: string;
  description?: string;
  local_exists: boolean;
};

export type SedimentKbSnap = {
  repos: SedimentKbRepo[];
  categories: SedimentKbCategory[];
  statusMap: Record<string, SedimentKbStatus>;
  diffStatus: Map<string, boolean> | null;
  listError: string;
  listLoading: boolean;
  manageError: string;
  manageFatal: boolean;
  syncingRepo: string;
};

export function emptySedimentKbSnap(): SedimentKbSnap {
  return {
    repos: [],
    categories: [],
    statusMap: {},
    diffStatus: null,
    listError: '',
    listLoading: false,
    manageError: '',
    manageFatal: false,
    syncingRepo: '',
  };
}

export const sedimentKbStore = createModuleStore<SedimentKbSnap>(emptySedimentKbSnap());

export function patchSedimentKb(partial: Partial<SedimentKbSnap>) {
  sedimentKbStore.set({ ...sedimentKbStore.getSnapshot(), ...partial });
}
