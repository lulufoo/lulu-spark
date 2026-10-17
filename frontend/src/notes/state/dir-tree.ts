import { createModuleStore } from '../../shared/module-store.ts';
import { normalizeDateKey } from '../../shared/utils.ts';
import type { NoteEntry } from './types.ts';

/**
 * The directory view groups documents by where their raw file lives: project folder first,
 * then the sub-folders of `raw_path`. A group key `dir:<node key>` names one tree node, the
 * same way a date key names one day in the timeline view.
 */
export const DIR_KEY_PREFIX = 'dir:';

export type DirDoc = { id: string; entry: NoteEntry; name: string };

export type DirNode = {
  /** `<project folder>` or `<project folder>/<sub>/<sub>`. */
  key: string;
  name: string;
  kind: 'project' | 'folder';
  children: DirNode[];
  /** Documents directly inside this node. */
  docs: DirDoc[];
};

export function isDirKey(value: unknown): value is string {
  return typeof value === 'string' && value.startsWith(DIR_KEY_PREFIX) && value.length > DIR_KEY_PREFIX.length;
}

export function dirKeyOf(nodeKey: string): string {
  return DIR_KEY_PREFIX + nodeKey;
}

export function nodeKeyOf(dirKey: string): string {
  return dirKey.slice(DIR_KEY_PREFIX.length);
}

/** A directory key, a date key, or ''. */
export function normalizeGroupKey(value: unknown): string {
  return isDirKey(value) ? value : normalizeDateKey(value);
}

/** Every node on the way to `nodeKey`, the node itself last: `a/b/c` → `a`, `a/b`, `a/b/c`. */
export function pathKeys(nodeKey: string): string[] {
  const parts = nodeKey.split('/').filter(Boolean);
  return parts.map((_, i) => parts.slice(0, i + 1).join('/'));
}

/** The project folder a node belongs to. */
export function projectOfNode(nodeKey: string): string {
  return nodeKey.split('/')[0] || '';
}

function splitRawPath(rawPath: unknown): { dirs: string[]; file: string } {
  const parts = (typeof rawPath === 'string' ? rawPath : '').split('/').filter(Boolean);
  const file = parts.pop() ?? '';
  return { dirs: parts, file };
}

/** The tree node whose own documents include `entry`. */
export function nodeKeyOfEntry(entry: NoteEntry): string {
  const { dirs } = splitRawPath(entry.raw_path);
  return [entry.folder || 'unknown', ...dirs].join('/');
}

/** Like main's Knowledge tree, a document shows as its file name (the title column can be empty). */
function docName(entry: NoteEntry, file: string): string {
  return file || entry.title || 'Untitled note';
}

/** Label of a group key for headings: the project folder shows its title. */
export function dirKeyLabel(dirKey: string, projectTitles: Record<string, string>): string {
  const [project = '', ...rest] = nodeKeyOf(dirKey).split('/');
  return [projectTitles[project] || project, ...rest].join(' / ');
}

function byName(a: { name: string }, b: { name: string }): number {
  return a.name.localeCompare(b.name);
}

function sortNode(node: DirNode) {
  node.children.sort(byName);
  node.docs.sort(byName);
  node.children.forEach(sortNode);
}

/**
 * Index entries → projects → sub-folders → documents, built in memory from `raw_path`
 * (no file system access). Projects without documents still appear, so the tree shows
 * every project; `projectTitles` lists them by folder.
 */
export function buildDirTree(
  data: Record<string, NoteEntry> | null,
  projectTitles: Record<string, string>,
): DirNode[] {
  const projects = new Map<string, DirNode>();
  const folders = new Map<string, DirNode>();

  const project = (folder: string): DirNode => {
    let node = projects.get(folder);
    if (!node) {
      node = { key: folder, name: projectTitles[folder] || folder, kind: 'project', children: [], docs: [] };
      projects.set(folder, node);
    }
    return node;
  };

  for (const folder of Object.keys(projectTitles)) project(folder);

  for (const [id, entry] of Object.entries(data || {})) {
    if (!entry) continue;
    const { dirs, file } = splitRawPath(entry.raw_path);
    let node = project(entry.folder || 'unknown');
    for (const dir of dirs) {
      const key = `${node.key}/${dir}`;
      let child = folders.get(key);
      if (!child) {
        child = { key, name: dir, kind: 'folder', children: [], docs: [] };
        folders.set(key, child);
        node.children.push(child);
      }
      node = child;
    }
    node.docs.push({ id, entry, name: docName(entry, file) });
  }

  const roots = [...projects.values()].sort(byName);
  roots.forEach(sortNode);
  return roots;
}

export function findDirNode(roots: DirNode[], nodeKey: string): DirNode | null {
  let level = roots;
  let found: DirNode | null = null;
  for (const key of pathKeys(nodeKey)) {
    found = level.find((node) => node.key === key) ?? null;
    if (!found) return null;
    level = found.children;
  }
  return found;
}

/** Project folders in the order of the directory view's dropdown: by title. */
export function sortedProjectFolders(projectTitles: Record<string, string>): string[] {
  return Object.keys(projectTitles).sort((a, b) => (projectTitles[a] || a).localeCompare(projectTitles[b] || b));
}

export type DirTreeSnap = { expanded: string[] };

/** Which nodes are open; reset when another workspace's index is loaded. */
export const dirTreeStore = createModuleStore<DirTreeSnap>({ expanded: [] });

export function resetDirTree() {
  dirTreeStore.set({ expanded: [] });
}

export function expandDirNodes(keys: string[]) {
  const { expanded } = dirTreeStore.getSnapshot();
  const next = [...expanded];
  for (const key of keys) if (!next.includes(key)) next.push(key);
  if (next.length !== expanded.length) dirTreeStore.set({ expanded: next });
}

export function toggleDirNodeOpen(key: string) {
  const { expanded } = dirTreeStore.getSnapshot();
  dirTreeStore.set({ expanded: expanded.includes(key) ? expanded.filter((k) => k !== key) : [...expanded, key] });
}
