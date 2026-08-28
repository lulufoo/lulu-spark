import { createModuleStore } from '../../shared/module-store.ts';

export const COMMIT_CHANGES_GROUPS = [
  { key: 'new', label: 'New' },
  { key: 'modified', label: 'Modified' },
  { key: 'renamed', label: 'Renamed' },
  { key: 'deleted', label: 'Deleted' },
  { key: 'conflicted', label: 'Conflict' },
] as const;

export type CommitChangesGroupKey = (typeof COMMIT_CHANGES_GROUPS)[number]['key'];

export type CommitChangesView = {
  loading: boolean;
  error: string;
  empty: boolean;
  groups: { key: CommitChangesGroupKey; label: string; files: string[] }[];
  ahead: number;
  message: string;
  canCommit: boolean;
  okLabel: string;
};

export function emptyCommitChangesView(): CommitChangesView {
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

export const commitChangesOpenStore = createModuleStore(false);
export const commitChangesViewStore = createModuleStore<CommitChangesView>(emptyCommitChangesView());

export function patchCommitChanges(partial: Partial<CommitChangesView>) {
  commitChangesViewStore.set({ ...commitChangesViewStore.getSnapshot(), ...partial });
}
