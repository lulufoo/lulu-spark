import { createModuleStore } from '../../../shared/module-store.ts';

export type SedimentKbRepo = {
  full_name: string;
  name: string;
  description: string;
  category_id: string;
  category_name: string;
};

export type SedimentKbCategory = {
  id: string;
  name: string;
};

export type SedimentKbSnap = {
  repos: SedimentKbRepo[];
  categories: SedimentKbCategory[];
  listError: string;
  listLoading: boolean;
  manageError: string;
  manageFatal: boolean;
};

export function emptySedimentKbSnap(): SedimentKbSnap {
  return {
    repos: [],
    categories: [],
    listError: '',
    listLoading: false,
    manageError: '',
    manageFatal: false,
  };
}

export const sedimentKbStore = createModuleStore<SedimentKbSnap>(emptySedimentKbSnap());

export function patchSedimentKb(partial: Partial<SedimentKbSnap>) {
  sedimentKbStore.set({ ...sedimentKbStore.getSnapshot(), ...partial });
}
