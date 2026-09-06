import { createModuleStore } from '../../../shared/module-store.ts';
import type { KbHidePatternRow } from '../../../knowledge/state/hide-pattern.ts';

export type KbHidePatternsSnap = {
  rows: KbHidePatternRow[];
  editingId: string;
  draft: string;
  addValue: string;
  error: string;
  busy: boolean;
};

export function emptyKbHidePatternsSnap(): KbHidePatternsSnap {
  return {
    rows: [],
    editingId: '',
    draft: '',
    addValue: '',
    error: '',
    busy: false,
  };
}

export const kbHidePatternsStore = createModuleStore<KbHidePatternsSnap>(emptyKbHidePatternsSnap());

export function patchKbHidePatterns(partial: Partial<KbHidePatternsSnap>) {
  kbHidePatternsStore.set({ ...kbHidePatternsStore.getSnapshot(), ...partial });
}
