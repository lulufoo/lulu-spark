import { createModuleStore } from '../../shared/module-store.ts';

export const WORKBENCH_COMMIT_GROUPS = [
  { key: 'new', label: 'New' },
  { key: 'modified', label: 'Modified' },
  { key: 'renamed', label: 'Renamed' },
  { key: 'deleted', label: 'Deleted' },
  { key: 'conflicted', label: 'Conflict' },
] as const;

export type WorkbenchCommitGroupKey = (typeof WORKBENCH_COMMIT_GROUPS)[number]['key'];

export type WorkbenchCommitView = {
  loading: boolean;
  error: string;
  empty: boolean;
  groups: { key: WorkbenchCommitGroupKey; label: string; files: string[] }[];
  ahead: number;
  message: string;
  canCommit: boolean;
  okLabel: string;
};

export function emptyWorkbenchCommitView(): WorkbenchCommitView {
  return {
    loading: false,
    error: '',
    empty: false,
    groups: [],
    ahead: 0,
    message: '',
    canCommit: true,
    okLabel: 'Commit',
  };
}

export const workbenchCommitOpenStore = createModuleStore(false);
export const workbenchCommitViewStore = createModuleStore<WorkbenchCommitView>(emptyWorkbenchCommitView());

export function patchWorkbenchCommit(partial: Partial<WorkbenchCommitView>) {
  workbenchCommitViewStore.set({ ...workbenchCommitViewStore.getSnapshot(), ...partial });
}
