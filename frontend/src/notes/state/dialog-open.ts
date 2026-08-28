import { createModuleStore } from '../../shared/module-store.ts';

export const deleteOpenStore = createModuleStore(false);
export const settleOpenStore = createModuleStore(false);
export const moveProjectOpenStore = createModuleStore(false);
export const commitOpenStore = createModuleStore(false);
export const commentOpenStore = createModuleStore(false);
