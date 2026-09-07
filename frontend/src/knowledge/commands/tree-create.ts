import * as api from '../../host/api.ts';
import {
  applyCreateInMemory,
  normalizeCreateName,
  type CreateKind,
} from '../state/tree-entry.ts';
import type { DirCacheEntry } from '../state/tree-rename.ts';
import type { TreeNode } from '../state/types.ts';

export type TreeCreateSession = {
  repo: string;
  nodes: TreeNode[];
  dirCache: Map<string, DirCacheEntry[]>;
};

export type TreeCreateResult =
  | { kind: 'invalid'; message: string }
  | { kind: 'applied'; nodes: TreeNode[]; selectedPath: string; openedPath: string };

export async function commitKbTreeCreate(
  session: TreeCreateSession,
  parentPath: string,
  createKind: CreateKind,
  name: string,
): Promise<TreeCreateResult> {
  const normalized = normalizeCreateName(name, createKind);
  if (!normalized) {
    return { kind: 'invalid', message: createKind === 'file' ? 'file must be .md' : 'invalid name' };
  }
  const data = (await api.createKbEntry(session.repo, parentPath, normalized, createKind)) as {
    path?: string;
  };
  const createdPath = typeof data.path === 'string' && data.path ? data.path : '';
  if (!createdPath) return { kind: 'invalid', message: 'create failed' };
  const applied = applyCreateInMemory(
    session.nodes,
    session.dirCache,
    parentPath,
    createdPath,
    createKind,
  );
  return {
    kind: 'applied',
    nodes: applied.nodes,
    selectedPath: applied.selectedPath,
    openedPath: createKind === 'file' ? createdPath : '',
  };
}
