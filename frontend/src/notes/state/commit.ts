import { createModuleStore } from '../../shared/module-store.ts';

export const COMMIT_GROUPS = [
  { key: 'new', label: 'New' },
  { key: 'modified', label: 'Modified' },
  { key: 'deleted', label: 'Deleted' },
  { key: 'renamed', label: 'Renamed' },
  { key: 'conflicted', label: 'Conflict' },
] as const;

export type CommitGroupKey = (typeof COMMIT_GROUPS)[number]['key'];

export type CommitFileGroup = {
  key: CommitGroupKey;
  label: string;
  files: string[];
};

export type CommitResultKind = '' | 'busy' | 'ok' | 'err';

export type CommitView = {
  loading: boolean;
  error: string;
  groups: CommitFileGroup[];
  empty: boolean;
  message: string;
  result: string;
  resultKind: CommitResultKind;
  canCommit: boolean;
  committing: boolean;
  revertAllConfirm: boolean;
  revertingPath: string | null;
};

export function emptyCommitView(): CommitView {
  return {
    loading: false,
    error: '',
    groups: [],
    empty: false,
    message: '',
    result: '',
    resultKind: '',
    canCommit: true,
    committing: false,
    revertAllConfirm: false,
    revertingPath: null,
  };
}

export const commitViewStore = createModuleStore<CommitView>(emptyCommitView());

export function patchCommit(partial: Partial<CommitView>) {
  commitViewStore.set({ ...commitViewStore.getSnapshot(), ...partial });
}
