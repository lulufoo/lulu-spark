import * as api from '../../host/api.ts';
import {
  applyRenameInMemory,
  nextRelativePath,
  type DirCacheEntry,
} from '../state/tree-rename.ts';
import type { TreeNode } from '../state/types.ts';

export { nextRelativePath };

export type TreeRenameSession = {
  repo: string;
  nodes: TreeNode[];
  dirCache: Map<string, DirCacheEntry[]>;
  selectedPath: string;
  openedPath: string;
};

export type TreeRenameResult =
  | { kind: 'noop' }
  | { kind: 'invalid'; message: string }
  | {
      kind: 'applied';
      nodes: TreeNode[];
      selectedPath: string;
      openedPath: string;
      openedChanged: boolean;
    };

export async function commitKbTreeRename(
  session: TreeRenameSession,
  from: string,
  name: string,
): Promise<TreeRenameResult> {
  const next = nextRelativePath(from, name);
  if (!next) return { kind: 'invalid', message: 'invalid name' };
  if (next === from) return { kind: 'noop' };
  const data = (await api.renameKbEntry(session.repo, from, name.trim())) as { path?: string };
  const to = typeof data.path === 'string' && data.path ? data.path : next;
  const applied = applyRenameInMemory(
    session.nodes,
    session.dirCache,
    session.selectedPath,
    session.openedPath,
    from,
    to,
  );
  return { kind: 'applied', ...applied };
}
