import type { ReadLaterEntry, ReadLaterFilter } from './types.ts';

export function sortEntries(entries: ReadLaterEntry[]) {
  return [...entries].sort((a, b) => {
    const aTime = Date.parse(a.saved_at ?? '') || 0;
    const bTime = Date.parse(b.saved_at ?? '') || 0;
    return bTime - aTime;
  });
}

export function filterEntries(entries: ReadLaterEntry[], filter: ReadLaterFilter) {
  if (filter === 'unread') {
    return sortEntries(entries.filter((entry) => !entry.read));
  }
  return sortEntries(entries);
}

export function emptyMessage(filter: ReadLaterFilter) {
  return filter === 'unread' ? 'No items to read later' : 'No items';
}

export function selectTop3Latest(entries: ReadLaterEntry[]) {
  return sortEntries(entries).slice(0, 3);
}
