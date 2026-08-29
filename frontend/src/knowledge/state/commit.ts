import { createModuleStore } from '../../shared/module-store.ts';

export const KB_COMMIT_GROUPS = [
  { key: 'new', label: 'New' },
  { key: 'modified', label: 'Modified' },
  { key: 'renamed', label: 'Renamed' },
  { key: 'deleted', label: 'Deleted' },
  { key: 'conflicted', label: 'Conflict' },
] as const;

export type KbCommitGroupKey = (typeof KB_COMMIT_GROUPS)[number]['key'];

export type KbCommitFileGroup = {
  key: KbCommitGroupKey;
  label: string;
  files: string[];
};

export type KbCommitView = {
  loading: boolean;
  error: string;
  empty: boolean;
  groups: KbCommitFileGroup[];
  ahead: number;
  message: string;
  result: string;
  resultKind: '' | 'busy' | 'ok' | 'err';
  canCommit: boolean;
  committing: boolean;
  okLabel: string;
  revertAllConfirm: boolean;
  revertAllDisabled: boolean;
  revertingPath: string | null;
};

export function emptyKbCommitView(): KbCommitView {
  return {
    loading: false,
    error: '',
    empty: false,
    groups: [],
    ahead: 0,
    message: '',
    result: '',
    resultKind: '',
    canCommit: true,
    committing: false,
    okLabel: 'Commit',
    revertAllConfirm: false,
    revertAllDisabled: false,
    revertingPath: null,
  };
}

export const kbCommitViewStore = createModuleStore<KbCommitView>(emptyKbCommitView());

export function patchKbCommit(partial: Partial<KbCommitView>) {
  kbCommitViewStore.set({ ...kbCommitViewStore.getSnapshot(), ...partial });
}
