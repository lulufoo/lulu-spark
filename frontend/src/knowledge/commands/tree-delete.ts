import * as api from '../../host/api.ts';
import { applyDeleteInMemory } from '../state/tree-entry.ts';
import type { DirCacheEntry } from '../state/tree-rename.ts';
import type { TreeNode } from '../state/types.ts';

export type TreeDeleteSession = {
  repo: string;
  nodes: TreeNode[];
  dirCache: Map<string, DirCacheEntry[]>;
  selectedPath: string;
  openedPath: string;
};

export type TreeDeleteResult = {
  nodes: TreeNode[];
  selectedPath: string;
  openedPath: string;
  openedChanged: boolean;
};

export async function commitKbTreeDelete(
  session: TreeDeleteSession,
  path: string,
): Promise<TreeDeleteResult> {
  await api.deleteKbEntry(session.repo, path);
  return applyDeleteInMemory(
    session.nodes,
    session.dirCache,
    session.selectedPath,
    session.openedPath,
    path,
  );
}
