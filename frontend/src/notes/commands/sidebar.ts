import { notifyState, state } from '../state/host.ts';
import { loadTitles } from './cards.ts';
import { parseHash, navigateToDateList } from '../../router/index.ts';
import type { NoteEntry } from '../state/types.ts';

export function buildGroups(indexData: Record<string, NoteEntry>) {
  const map = new Map<string, { id: string; entry: NoteEntry }[]>();
  for (const [id, entry] of Object.entries(indexData)) {
    if (!entry || typeof entry.created_at !== 'string') continue;
    const date = entry.created_at.slice(0, 8);
    if (!map.has(date)) map.set(date, []);
    map.get(date)!.push({ id, entry });
  }
  return [...map.entries()]
    .sort((a, b) => b[0].localeCompare(a[0]))
    .map(([date, entries]) => ({
      date,
      entries: entries.sort((a, b) => String(b.entry.created_at).localeCompare(String(a.entry.created_at))),
    }));
}

export function applyListFilters() {
  let groups = state.index.groupedByDate;
  const topic = state.ui.activeTopic;
  if (topic) {
    groups = groups
      .map(({ date, entries }) => ({
        date,
        entries: entries.filter(({ entry }) => (entry.common_path?.split('/')[0] || 'unknown') === topic),
      }))
      .filter(({ entries }) => entries.length > 0);
  }
  const tagKey = state.ui.activeTagKey;
  if (tagKey) {
    groups = groups
      .map(({ date, entries }) => ({
        date,
        entries: entries.filter(({ entry }) => entry.tag_keys?.includes(tagKey)),
      }))
      .filter(({ entries }) => entries.length > 0);
  }
  state.index.filteredGroups = groups;
}

export function renderTagFilterChip() {
  notifyState();
}

export function selectTag(key: string) {
  if (state.ui.activeTagKey === key) {
    clearTagFilter();
    return;
  }
  state.ui.activeTagKey = key;
  _refreshFilteredList();
}

export function clearTagFilter() {
  state.ui.activeTagKey = null;
  _refreshFilteredList();
}

function _refreshFilteredList() {
  applyListFilters();
  if (state.index.filteredGroups.length > 0) {
    selectDate(state.index.filteredGroups[0].date);
  } else {
    state.ui.activeDate = null;
    notifyState();
  }
}

export function selectDate(date: string) {
  const route = parseHash(typeof window !== 'undefined' ? window.location.hash : '');
  if (route.name === 'workbench') {
    const params = route.params as { note?: string };
    const hasNote = Boolean(params.note);
    const creating = Boolean(state.viewer?.createSession);
    if (hasNote || creating) {
      if (creating) state.viewer.createSession = null;
      navigateToDateList(date);
      return;
    }
  }

  state.ui.activeDate = date;
  sessionStorage.setItem('cta_active_date', date);
  notifyState();

  const group = state.index.filteredGroups.find((g) => g.date === date);
  if (!group) return;
  loadTitles(group.entries, date);
}

export function selectTopic(key: string | null) {
  state.ui.activeTopic = key;
  applyListFilters();
  if (state.index.filteredGroups.length > 0) {
    selectDate(state.index.filteredGroups[0].date);
  } else {
    state.ui.activeDate = null;
    notifyState();
  }
}
