// @ts-nocheck — host/state snapshots stay unchecked; do not type this file alone.
import { notifyState, state } from '../state/host.ts';
import { IMPORTANCE_CYCLE } from '../../host/constants.ts';
import { slugToTitle, filenameFromPath } from '../../shared/utils.ts';
import * as api from '../../host/api.ts';

type NoteEntry = {
  common_path: string;
  layers?: string[];
  done?: boolean | undefined;
  importance?: string;
  translations?: { zh?: string };
};

function asRecord(entry: NoteEntry): Record<string, unknown> {
  return entry as unknown as Record<string, unknown>;
}

export async function cycleImportance(entry: NoteEntry) {
  const cur = entry.importance;
  const idx = IMPORTANCE_CYCLE.indexOf(cur);
  const next = IMPORTANCE_CYCLE[(idx + 1) % IMPORTANCE_CYCLE.length];
  try {
    const data = await api.setImportance(entry.common_path, next);
    if (!data.ok) return;
    entry.importance = next;
    notifyState();
  } catch (e) {
    console.error('cycleImportance failed', e);
  }
}

export async function toggleDone(entry: NoteEntry) {
  const newDone = !entry.done;
  try {
    const data = await api.setDone(entry.common_path, newDone);
    if (!data.ok) return;
    entry.done = newDone || undefined;
    notifyState();
  } catch (e) {
    console.error('toggleDone failed', e);
  }
}

export async function loadTitles(entries: { id: string; entry: NoteEntry }[], date: string) {
  if (!state.index.titleCache.has(date)) {
    state.index.titleCache.set(date, new Map());
  }
  const cache = state.index.titleCache.get(date);
  const pending = entries.filter(({ id }) => !cache.has(id));
  if (pending.length === 0) {
    if (state.ui.activeDate === date) notifyState();
    return;
  }

  await Promise.all(
    pending.map(async ({ id, entry }) => {
      if (!entry.layers?.includes('raw')) {
        const title = slugToTitle(filenameFromPath(entry.common_path));
        cache.set(id, title);
        return;
      }

      try {
        const titlePath = entry.translations?.zh || entry.common_path;
        const text = await api.fetchFileContent('raw', titlePath);
        const h1Match = text.match(/^#\s+(.+)/m);
        const title = h1Match ? h1Match[1].trim() : slugToTitle(filenameFromPath(entry.common_path));
        cache.set(id, title);
      } catch {
        asRecord(entry)._unreachable_raw = true;
        cache.set(id, slugToTitle(filenameFromPath(entry.common_path)));
      }
    }),
  );

  if (state.ui.activeDate === date) notifyState();
}
