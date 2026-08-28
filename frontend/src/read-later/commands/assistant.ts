import { loadReadLaterEntries } from './list.ts';
import { selectTop3Latest } from '../state/selectors.ts';
import type { ReadLaterEntry } from '../state/types.ts';

export type { ReadLaterEntry };
export { selectTop3Latest };

export async function loadAssistantEntries() {
  return loadReadLaterEntries();
}
