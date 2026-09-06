import { createModuleStore } from '../../shared/module-store.ts';
import type { TreeNode } from './types.ts';

export type KnowledgeTreeSnap = {
  nodes: TreeNode[];
  selectedPath: string;
  renamingPath: string;
  error: string;
  version: number;
};

export const knowledgeTreeStore = createModuleStore<KnowledgeTreeSnap>({
  nodes: [],
  selectedPath: '',
  renamingPath: '',
  error: '',
  version: 0,
});

export function publishKnowledgeTree(
  nodes: TreeNode[],
  selectedPath: string,
  error = '',
  renamingPath = '',
) {
  knowledgeTreeStore.set({
    nodes,
    selectedPath,
    renamingPath,
    error,
    version: knowledgeTreeStore.getSnapshot().version + 1,
  });
}
