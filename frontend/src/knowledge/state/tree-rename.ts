import type { TreeNode } from './types.ts';

export type DirCacheEntry = { name: string; relative_path: string; is_dir: boolean };

export function isValidEntryName(name: string): boolean {
  const trimmed = name.trim();
  if (!trimmed || trimmed === '.' || trimmed === '..') return false;
  if (trimmed.includes('/') || trimmed.includes('\\') || trimmed.includes('\0')) return false;
  if (trimmed.includes('..')) return false;
  return true;
}

export function nextRelativePath(from: string, name: string): string | null {
  const trimmed = name.trim();
  if (!isValidEntryName(trimmed)) return null;
  const src = from.trim();
  if (!src) return null;
  const slash = src.lastIndexOf('/');
  return slash === -1 ? trimmed : `${src.slice(0, slash)}/${trimmed}`;
}

export function rewriteRelativePath(path: string, from: string, to: string): string {
  if (!path || !from) return path;
  if (path === from) return to;
  if (path.startsWith(`${from}/`)) return `${to}${path.slice(from.length)}`;
  return path;
}

function basename(path: string): string {
  const slash = path.lastIndexOf('/');
  return slash === -1 ? path : path.slice(slash + 1);
}

function rewriteNode(node: TreeNode, from: string, to: string): TreeNode {
  const relative_path = rewriteRelativePath(node.relative_path, from, to);
  return {
    ...node,
    name: relative_path === to ? basename(to) : node.name,
    relative_path,
    children: node.children.map((child) => rewriteNode(child, from, to)),
  };
}

export function rewriteTreeAfterRename(nodes: TreeNode[], from: string, to: string): TreeNode[] {
  return nodes.map((node) => rewriteNode(node, from, to));
}

export function rewriteDirCache(
  cache: Map<string, DirCacheEntry[]>,
  from: string,
  to: string,
): void {
  const next = new Map<string, DirCacheEntry[]>();
  for (const [key, entries] of cache) {
    next.set(
      rewriteRelativePath(key, from, to),
      entries.map((entry) => ({
        ...entry,
        name: entry.relative_path === from ? basename(to) : entry.name,
        relative_path: rewriteRelativePath(entry.relative_path, from, to),
      })),
    );
  }
  cache.clear();
  for (const [key, entries] of next) {
    cache.set(key, entries);
  }
}

export function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  if (target.closest('.knowledge-doc-tree-rename')) return true;
  const el = target as HTMLElement;
  const tag = (el.tagName || '').toLowerCase();
  if (tag === 'input' || tag === 'textarea' || tag === 'select') return true;
  return Boolean(el.isContentEditable);
}

export function resolveRenamePathFromKeydown({
  key,
  repeat,
  isComposing,
  metaKey,
  ctrlKey,
  altKey,
  target,
  host,
  renamingPath,
  selectedPath,
}: {
  key: string;
  repeat?: boolean;
  isComposing?: boolean;
  metaKey?: boolean;
  ctrlKey?: boolean;
  altKey?: boolean;
  target: EventTarget | null;
  host: Element | null;
  renamingPath: string;
  selectedPath: string;
}): string {
  if (key !== 'Enter' || repeat || isComposing || metaKey || ctrlKey || altKey) return '';
  if (renamingPath || isTypingTarget(target)) return '';
  const el = target instanceof Element ? target : null;
  const focusLost = el === document.body || el === document.documentElement;
  if (host && el && !host.contains(el) && !focusLost) return '';
  const nodeEl = el?.closest?.('.knowledge-doc-tree-node') as HTMLElement | null;
  return nodeEl?.dataset.relativePath || selectedPath || '';
}

export function applyRenameInMemory(
  nodes: TreeNode[],
  dirCache: Map<string, DirCacheEntry[]>,
  selectedPath: string,
  openedPath: string,
  from: string,
  to: string,
): { nodes: TreeNode[]; selectedPath: string; openedPath: string; openedChanged: boolean } {
  rewriteDirCache(dirCache, from, to);
  const nextOpened = rewriteRelativePath(openedPath, from, to);
  return {
    nodes: rewriteTreeAfterRename(nodes, from, to),
    selectedPath: rewriteRelativePath(selectedPath, from, to),
    openedPath: nextOpened,
    openedChanged: nextOpened !== openedPath,
  };
}
