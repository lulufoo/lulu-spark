import { createModuleStore } from '../../shared/module-store.ts';

/** Where the popover sits: fixed, right-aligned under the toolbar tag button. */
export type TagsPopoverSnap = {
  open: boolean;
  top: number;
  right: number;
};

export const tagsPopoverStore = createModuleStore<TagsPopoverSnap>({
  open: false,
  top: 0,
  right: 0,
});
