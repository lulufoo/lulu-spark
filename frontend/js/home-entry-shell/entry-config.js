/**
 * EntryConfig baseline for the home-entry shell.
 * Display metadata only — content adapters live in ContentRegistry.
 */

const BASELINE_ENTRIES = [
  {
    id: 'read-later',
    contentKey: 'read-later',
    title: 'Read Later',
    overlayTitle: 'Read Later',
  },
  {
    id: 'plan-task',
    contentKey: 'plan-task',
    title: 'Todos',
    overlayTitle: 'Todos',
  },
  {
    id: 'notes',
    contentKey: 'notes',
    title: 'Notes Assistant',
    overlayTitle: 'Notes Assistant',
  },
  {
    id: 'builders',
    contentKey: 'builders',
    title: 'Builders',
    overlayTitle: 'Builders',
  },
];

/** @returns {ReadonlyArray<{ id: string, contentKey: string, title: string, overlayTitle: string }>} */
export function getBaselineEntries() {
  return BASELINE_ENTRIES.map((entry) => ({ ...entry }));
}

/** @returns {ReadonlyArray<{ id: string, contentKey: string, title: string, overlayTitle: string }>} */
export function createEntryConfig() {
  return getBaselineEntries();
}
