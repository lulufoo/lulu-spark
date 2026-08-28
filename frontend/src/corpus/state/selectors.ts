import { repoShortName } from '../../shared/utils.ts';
import { getKbHidePattern, shouldHideEntry } from './hide-pattern.ts';
import type { RepoPickerOption, TreeNode } from './types.ts';

export function formatRepoMenuLabel(shortName: string, count: number | null | undefined) {
  if (count == null || Number.isNaN(count)) return shortName;
  return `${shortName} · ${Math.floor(count)}`;
}

export function buildRepoPickerOptions(
  repos: Array<{ full_name: string }>,
  countByRepo: Map<string, number> = new Map(),
): RepoPickerOption[] {
  return repos.map((r) => {
    const fullName = r.full_name || '';
    const short = repoShortName(fullName);
    const count = countByRepo.has(fullName) ? countByRepo.get(fullName) : null;
    return {
      value: fullName,
      label: formatRepoMenuLabel(short, count ?? null),
      title: fullName,
    };
  });
}

export function buildTreeNodes(
  entries: Array<{ name?: string; relative_path?: string; is_dir?: boolean }>,
  _parentPath: string,
): TreeNode[] {
  const hidePattern = getKbHidePattern();
  const seen = new Set<string>();
  const nodes: TreeNode[] = [];
  for (const entry of entries || []) {
    const name = entry.name || (entry.relative_path || '').split('/').pop() || '';
    if (shouldHideEntry(name, hidePattern)) continue;
    const relative_path = entry.relative_path || entry.name || '';
    if (!relative_path || seen.has(relative_path)) continue;
    seen.add(relative_path);
    nodes.push({
      name: entry.name || relative_path.split('/').pop() || relative_path,
      relative_path,
      is_dir: Boolean(entry.is_dir),
      expanded: false,
      loaded: false,
      children: [],
    });
  }
  nodes.sort((a, b) => {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
  return nodes;
}
