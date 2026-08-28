import { createModuleStore } from '../../shared/module-store.ts';
import type { TreeNode } from './types.ts';

export type CorpusTreeSnap = {
  nodes: TreeNode[];
  selectedPath: string;
  error: string;
  version: number;
};

export const corpusTreeStore = createModuleStore<CorpusTreeSnap>({
  nodes: [],
  selectedPath: '',
  error: '',
  version: 0,
});

export function publishCorpusTree(nodes: TreeNode[], selectedPath: string, error = '') {
  corpusTreeStore.set({
    nodes,
    selectedPath,
    error,
    version: corpusTreeStore.getSnapshot().version + 1,
  });
}
