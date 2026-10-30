import { createModuleStore } from '../../shared/module-store.ts';

export type MoveProjectResultKind = '' | 'loading' | 'hint' | 'ok' | 'err';

export type MoveProjectItem = {
  project_id: string;
  project_name: string;
  description: string;
  is_default: boolean;
};

export type MoveWorkspaceItem = {
  workspace_id: string;
  workspace_name: string;
};

export type MoveProjectView = {
  currentProject: string;
  sourceWorkspaceId: string;
  selectedWorkspaceId: string;
  workspaces: MoveWorkspaceItem[];
  items: MoveProjectItem[];
  result: string;
  resultKind: MoveProjectResultKind;
  busy: boolean;
};

export function emptyMoveProjectView(): MoveProjectView {
  return {
    currentProject: '',
    sourceWorkspaceId: '',
    selectedWorkspaceId: '',
    workspaces: [],
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
