import { isValidEntryName } from './tree-rename.ts';
import type { DirCacheEntry } from './tree-rename.ts';
import type { TreeNode } from './types.ts';

export const CREATING_PATH = '__creating__';

export type CreateKind = 'file' | 'dir';

export function parentPathForAdd(relativePath: string, isDir: boolean): string {
  if (!relativePath || isDir) return relativePath.trim();
  const slash = relativePath.lastIndexOf('/');
  return slash === -1 ? '' : relativePath.slice(0, slash);
}

export function joinEntryPath(parent: string, name: string): string {
  const p = parent.trim();
  const n = name.trim();
  if (!p) return n;
  if (!n) return p;
  return `${p}/${n}`;
}

export function normalizeCreateFileName(name: string): string | null {
  const trimmed = name.trim();
  if (!isValidEntryName(trimmed)) return null;
  if (/\.md$/i.test(trimmed)) return `${trimmed.slice(0, -3)}.md`;
  if (trimmed.includes('.')) return null;
  return `${trimmed}.md`;
}

export function normalizeCreateName(name: string, kind: CreateKind): string | null {
  if (kind === 'file') return normalizeCreateFileName(name);
  const trimmed = name.trim();
  return isValidEntryName(trimmed) ? trimmed : null;
}

export function defaultCreateName(kind: CreateKind): string {
  return kind === 'file' ? 'untitled.md' : 'untitled';
}

export function creatingPathFor(parentPath: string): string {
  return joinEntryPath(parentPath, CREATING_PATH);
}

function sortNodes(nodes: TreeNode[]): TreeNode[] {
  return [...nodes].sort((a, b) => {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
}

function insertChild(nodes: TreeNode[], parentPath: string, child: TreeNode): TreeNode[] {
  if (!parentPath) return sortNodes([...nodes, child]);
  return nodes.map((node) => {
    if (node.relative_path !== parentPath) {
      if (!node.children.length) return node;
      return { ...node, children: insertChild(node.children, parentPath, child) };
    }
    return {
      ...node,
      expanded: true,
      loaded: true,
      children: sortNodes([...node.children, child]),
    };
  });
}

export function makeDraftNode(parentPath: string, kind: CreateKind): TreeNode {
  return {
    name: defaultCreateName(kind),
    relative_path: creatingPathFor(parentPath),
    is_dir: kind === 'dir',
    expanded: false,
    loaded: false,
    children: [],
  };
}

export function insertDraftNode(
  nodes: TreeNode[],
  parentPath: string,
  kind: CreateKind,
): TreeNode[] {
  return insertChild(nodes, parentPath, makeDraftNode(parentPath, kind));
}

export function removeTreeNode(nodes: TreeNode[], path: string): TreeNode[] {
  return nodes
    .filter((node) => node.relative_path !== path)
    .map((node) =>
      node.children.length ? { ...node, children: removeTreeNode(node.children, path) } : node,
    );
}

export function pathIsInside(path: string, ancestor: string): boolean {
  if (!path || !ancestor) return path === ancestor;
  return path === ancestor || path.startsWith(`${ancestor}/`);
}

function insertCacheEntry(
  cache: Map<string, DirCacheEntry[]>,
  parentPath: string,
  entry: DirCacheEntry,
) {
  const current = cache.get(parentPath) || [];
  cache.set(
    parentPath,
    [...current.filter((item) => item.relative_path !== entry.relative_path), entry],
  );
}

export function applyCreateInMemory(
  nodes: TreeNode[],
  dirCache: Map<string, DirCacheEntry[]>,
  parentPath: string,
  createdPath: string,
  kind: CreateKind,
): { nodes: TreeNode[]; selectedPath: string } {
  const name = createdPath.split('/').pop() || createdPath;
  const next = insertChild(removeTreeNode(nodes, creatingPathFor(parentPath)), parentPath, {
    name,
    relative_path: createdPath,
    is_dir: kind === 'dir',
    expanded: false,
    loaded: kind === 'dir',
    children: [],
  });
  insertCacheEntry(dirCache, parentPath, {
    name,
    relative_path: createdPath,
    is_dir: kind === 'dir',
  });
  return { nodes: next, selectedPath: createdPath };
}

export function applyDeleteInMemory(
  nodes: TreeNode[],
  dirCache: Map<string, DirCacheEntry[]>,
  selectedPath: string,
  openedPath: string,
  path: string,
): { nodes: TreeNode[]; selectedPath: string; openedPath: string; openedChanged: boolean } {
  const parent = parentPathForAdd(path, false);
  const current = dirCache.get(parent) || [];
  dirCache.set(
    parent,
    current.filter((item) => item.relative_path !== path),
  );
  for (const key of [...dirCache.keys()]) {
    if (key === path || key.startsWith(`${path}/`)) dirCache.delete(key);
  }
  const nextSelected = pathIsInside(selectedPath, path) ? parent : selectedPath;
  const nextOpened = pathIsInside(openedPath, path) ? '' : openedPath;
  return {
    nodes: removeTreeNode(nodes, path),
    selectedPath: nextSelected,
    openedPath: nextOpened,
    openedChanged: nextOpened !== openedPath,
  };
}
