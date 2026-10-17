import { createModuleStore } from '../../../shared/module-store.ts';
import type { HidePatternRow } from '../../../host/api.ts';

export type HidePatternsSnap = {
  /** Workspace whose Hidden files rules the tab shows. */
  workspaceId: string;
  rows: HidePatternRow[];
  editingId: string;
  draft: string;
  addValue: string;
  error: string;
  busy: boolean;
};

export function emptyHidePatternsSnap(): HidePatternsSnap {
  return {
    workspaceId: '',
    rows: [],
    editingId: '',
    draft: '',
    addValue: '',
    error: '',
    busy: false,
  };
}

export const hidePatternsStore = createModuleStore<HidePatternsSnap>(emptyHidePatternsSnap());

export function patchHidePatterns(partial: Partial<HidePatternsSnap>) {
  hidePatternsStore.set({ ...hidePatternsStore.getSnapshot(), ...partial });
}
