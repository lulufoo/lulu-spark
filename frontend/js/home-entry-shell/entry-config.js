/**
 * EntryConfig baseline for the home-entry shell.
 * Display metadata only — content adapters live in ContentRegistry.
 */

/** @typedef {{ id: string, contentKey: string, title: string, overlayTitle: string }} EntryConfig */

/** @type {ReadonlyArray<{ key: string, title: string }>} */
const BASELINE = [
  { key: 'read-later', title: 'Read Later' },
  { key: 'plan-task', title: 'Todos' },
  { key: 'notes', title: 'Notes Assistant' },
  { key: 'builders', title: 'Builders' },
];

/** @returns {EntryConfig[]} */
export function getBaselineEntries() {
  return BASELINE.map(({ key, title }) => ({
    id: key,
    contentKey: key,
    title,
    overlayTitle: title,
  }));
}

/** @returns {EntryConfig[]} */
export function createEntryConfig() {
  return getBaselineEntries();
}
