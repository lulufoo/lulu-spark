import { createModuleStore } from '../../../shared/module-store.ts';

export type BoardPatSnap = {
  /** `null` until a status read finishes. */
  configured: boolean | null;
  message: string;
  busy: boolean;
};

export function emptyBoardPatSnap(): BoardPatSnap {
  return { configured: null, message: '', busy: false };
}

export const boardPatStore = createModuleStore<BoardPatSnap>(emptyBoardPatSnap());

export function patchBoardPat(partial: Partial<BoardPatSnap>) {
  boardPatStore.set({ ...boardPatStore.getSnapshot(), ...partial });
}
