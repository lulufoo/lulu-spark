import { createModuleStore } from '../../shared/module-store.ts';

export const SPARK_COMMIT_GROUPS = [
  { key: 'new', label: 'New' },
  { key: 'modified', label: 'Modified' },
  { key: 'renamed', label: 'Renamed' },
  { key: 'deleted', label: 'Deleted' },
  { key: 'conflicted', label: 'Conflict' },
] as const;

export type SparkCommitGroupKey = (typeof SPARK_COMMIT_GROUPS)[number]['key'];

export type SparkCommitView = {
  loading: boolean;
  error: string;
  empty: boolean;
  groups: { key: SparkCommitGroupKey; label: string; files: string[] }[];
  ahead: number;
  message: string;
  canCommit: boolean;
  okLabel: string;
};

export function emptySparkCommitView(): SparkCommitView {
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

export const sparkCommitOpenStore = createModuleStore(false);
export const sparkCommitViewStore = createModuleStore<SparkCommitView>(emptySparkCommitView());

export function patchSparkCommit(partial: Partial<SparkCommitView>) {
  sparkCommitViewStore.set({ ...sparkCommitViewStore.getSnapshot(), ...partial });
}
