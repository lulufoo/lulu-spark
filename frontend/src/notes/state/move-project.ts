import { createModuleStore } from '../../shared/module-store.ts';

export type MoveProjectResultKind = '' | 'loading' | 'hint' | 'ok' | 'err';

export type MoveProjectItem = {
  proj: string;
  title: string;
  desc: string;
};

export type MoveProjectView = {
  currentProject: string;
  items: MoveProjectItem[];
  result: string;
  resultKind: MoveProjectResultKind;
  busy: boolean;
};

export function emptyMoveProjectView(): MoveProjectView {
  return {
    currentProject: '',
    items: [],
    result: '',
    resultKind: '',
    busy: false,
  };
}

export const moveProjectViewStore = createModuleStore<MoveProjectView>(emptyMoveProjectView());

export function patchMoveProject(partial: Partial<MoveProjectView>) {
  moveProjectViewStore.set({ ...moveProjectViewStore.getSnapshot(), ...partial });
}
