import { createModuleStore } from '../../shared/module-store.ts';

/** The Project chooser that opens when "+" is pressed with no Project selected (timeline All). */
export type CreateTargetSnap = {
  open: boolean;
  /** Project folder picked in the chooser. */
  project: string;
};

export const createTargetStore = createModuleStore<CreateTargetSnap>({ open: false, project: '' });
