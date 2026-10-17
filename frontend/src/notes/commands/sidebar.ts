import { notifyState, state } from '../state/host.ts';
import { loadTitles } from './cards.ts';
import { isTimelineView } from './workspace.ts';
import { parseHash, navigateToDateList } from '../../router/index.ts';
import { dateKeyOf, UNDATED_KEY } from '../../shared/utils.ts';
import type { NoteEntry } from '../state/types.ts';

/**
 * Group documents by day, newest day first. Documents without a usable `created_at` are not
 * dropped: they form one `Undated` group after every date, ordered by title.
 */
export function buildGroups(indexData: Record<string, NoteEntry>) {
  const map = new Map<string, { id: string; entry: NoteEntry }[]>();
  for (const [id, entry] of Object.entries(indexData)) {
    if (!entry) continue;
    const date = dateKeyOf(entry.created_at);
    if (!map.has(date)) map.set(date, []);
    map.get(date)!.push({ id, entry });
  }
  const rank = (date: string) => (date === UNDATED_KEY ? 0 : 1);
  return [...map.entries()]
    .sort((a, b) => rank(b[0]) - rank(a[0]) || b[0].localeCompare(a[0]))
    .map(([date, entries]) => ({
      date,
      entries: date === UNDATED_KEY
        ? entries.sort(compareByTitle)
        : entries.sort((a, b) => String(b.entry.created_at).localeCompare(String(a.entry.created_at))),
    }));
}

function compareByTitle(a: { id: string; entry: NoteEntry }, b: { id: string; entry: NoteEntry }) {
  return (a.entry.title || '').localeCompare(b.entry.title || '') || a.id.localeCompare(b.id);
}

/** Directory view filters by project; the timeline view lists every document of the workspace. */
export function applyListFilters() {
  let groups = state.index.groupedByDate;
  const topic = state.ui.activeTopic;
  if (topic && !isTimelineView()) {
    groups = groups
      .map(({ date, entries }) => ({
        date,
        entries: entries.filter(({ entry }) => (entry.folder || 'unknown') === topic),
      }))
      .filter(({ entries }) => entries.length > 0);
  }
  state.index.filteredGroups = groups;
}

export function selectDate(date: string) {
  const route = parseHash(typeof window !== 'undefined' ? window.location.hash : '');
  if (route.name === 'spark') {
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
