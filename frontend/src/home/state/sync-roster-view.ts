import type { SyncProgressEntry } from '../../host/api.ts';

/** The four places a document of the list can be in, in the order the panel shows them. */
export type SyncGroupKey = 'failed' | 'running' | 'done' | 'not_started';

export type SyncGroup = {
  key: SyncGroupKey;
  /** The heading of the group. */
  label: string;
  /** The documents of the group, in the order the host listed them. */
  entries: SyncProgressEntry[];
};

/** How many rows a group shows before the rest fold into "N more". */
export const GROUP_VISIBLE_ROWS = 5;

const GROUPS: ReadonlyArray<{ key: SyncGroupKey; label: string }> = [
  { key: 'failed', label: 'Failed' },
  { key: 'running', label: 'In progress' },
  { key: 'done', label: 'Done' },
  { key: 'not_started', label: 'Not started' },
];

/**
 * Sorts the host's list into groups: Failed first (what the user may act on), then In progress,
 * Done and Not started. A group with no document is left out, so Failed shows only when there is
 * a failure. Nothing is added or lost: every document is in exactly one group.
 */
export function groupRoster(entries: readonly SyncProgressEntry[]): SyncGroup[] {
  return GROUPS.map(({ key, label }) => ({
    key,
    label,
    entries: entries.filter((entry) => entry.state === key),
  })).filter((group) => group.entries.length > 0);
}

/** A long group shows its first rows and tells how many more there are; opened, it shows all. */
export function foldGroup(
  entries: readonly SyncProgressEntry[],
  opened: boolean,
): { shown: SyncProgressEntry[]; hidden: number } {
  if (opened || entries.length <= GROUP_VISIBLE_ROWS) return { shown: [...entries], hidden: 0 };
  return { shown: entries.slice(0, GROUP_VISIBLE_ROWS), hidden: entries.length - GROUP_VISIBLE_ROWS };
}

/**
 * What a row says when the pointer rests on it: the whole name, the id, and for a document that
 * failed, the reason in the words the host gave.
 */
export function rowHover(entry: SyncProgressEntry): string {
  const lines = [entry.name, entry.id];
  if (entry.state === 'failed' && entry.reason) lines.push(entry.reason);
  return lines.join('\n');
}

/** Whether some document of the list failed (the panel then names them, so its summary need not). */
export function hasFailedDocs(entries: readonly SyncProgressEntry[]): boolean {
  return entries.some((entry) => entry.state === 'failed');
}
