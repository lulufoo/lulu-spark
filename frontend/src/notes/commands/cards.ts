import { getEntryId, notifyState, state } from '../state/host.ts';
import { IMPORTANCE_CYCLE } from '../../host/constants.ts';
import * as api from '../../host/api.ts';
import { exposedCategoryOptions } from '../../host/doc-categories-catalog.ts';
import { openAnchoredListSelect } from '../../shared/floating-list-select.ts';
import type { NoteEntry } from '../state/types.ts';

function asRecord(entry: NoteEntry): Record<string, unknown> {
  return entry as unknown as Record<string, unknown>;
}

export async function cycleImportance(entry: NoteEntry) {
  const cur = entry.importance;
  const idx = IMPORTANCE_CYCLE.indexOf(cur);
  const next = IMPORTANCE_CYCLE[(idx + 1) % IMPORTANCE_CYCLE.length];
  try {
    const data = await api.setImportance(getEntryId(entry) || '', next);
    if (!data.ok) return;
    entry.importance = next;
    notifyState();
  } catch (e) {
    console.error('cycleImportance failed', e);
  }
}

export async function setEntrySourceType(entry: NoteEntry, sourceType: string) {
  if (entry.source_type === sourceType) return;
  try {
    const data = await api.setDocSourceType(getEntryId(entry) || '', sourceType);
    if (!data.ok) return;
    entry.source_type = sourceType;
    notifyState();
  } catch (e) {
    console.error('setEntrySourceType failed', e);
  }
}

export function openSourceTypePicker(trigger: HTMLElement, entry: NoteEntry) {
  const options = exposedCategoryOptions();
  if (!options.length) return;
  openAnchoredListSelect({
    trigger,
    options,
    value: entry.source_type || '',
    onSelect: (next) => {
      void setEntrySourceType(entry, next);
    },
  });
}

export async function toggleDone(entry: NoteEntry) {
  const newDone = !entry.done;
  try {
    const data = await api.setDone(getEntryId(entry) || '', newDone);
    if (!data.ok) return;
    entry.done = newDone || undefined;
    notifyState();
  } catch (e) {
    console.error('toggleDone failed', e);
  }
}

export async function loadTitles(entries: { id: string; entry: NoteEntry }[], date: string) {
  const cache = state.index.titleCache.get(date) ?? new Map<string, string>();
  state.index.titleCache.set(date, cache);
  const pending = entries.filter(({ id }) => !cache.has(id));
  if (pending.length === 0) {
    if (state.ui.activeDate === date) notifyState();
    return;
  }

  await Promise.all(
    pending.map(async ({ id, entry }) => {
      if (!entry.layers?.includes('raw')) {
        const title = entry.title || 'Untitled note';
        cache.set(id, title);
        return;
      }

      try {
        const lang = entry.translations?.zh ? 'zh' : undefined;
        const text = await api.fetchFileContent('raw', id, lang);
        const h1Match = text.match(/^#\s+(.+)/m);
        const title = h1Match ? h1Match[1].trim() : (entry.title || 'Untitled note');
        cache.set(id, title);
      } catch {
        asRecord(entry)._unreachable_raw = true;
        cache.set(id, entry.title || 'Untitled note');
      }
    }),
  );

  if (state.ui.activeDate === date) notifyState();
}
