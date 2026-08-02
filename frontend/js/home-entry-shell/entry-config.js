/**
 * EntryConfig baseline for the home-entry shell.
 * Display metadata only — content adapters live in ContentRegistry.
 */

/**
 * @typedef {{
 *   id: string,
 *   contentKey: string,
 *   title: string,
 *   overlayTitle: string,
 *   fabClass: string,
 *   fabIconClass: string,
 *   iconPaths: string,
 *   panelWidth: number,
 *   panelHeight: number,
 * }} EntryConfig
 */

/** @type {ReadonlyArray<Omit<EntryConfig, 'id' | 'contentKey' | 'overlayTitle'> & { key: string }>} */
const BASELINE = [
  {
    key: 'read-later',
    title: 'Read Later',
    fabClass: 'rl-assistant-fab',
    fabIconClass: 'rl-assistant-fab-icon',
    iconPaths:
      '<path fill="currentColor" d="M11.2 2.2a.9.9 0 0 1 .6 0l1.1 4.4 4.4 1.1a.9.9 0 0 1 0 1.7l-4.4 1.1-1.1 4.4a.9.9 0 0 1-1.7 0l-1.1-4.4-4.4-1.1a.9.9 0 0 1 0-1.7l4.4-1.1 1.1-4.4z"/>' +
      '<path fill="currentColor" d="M18.2 13.8a.7.7 0 0 1 .5 0l.8 3.2 3.2.8a.7.7 0 0 1 0 1.3l-3.2.8-.8 3.2a.7.7 0 0 1-1.3 0l-.8-3.2-3.2-.8a.7.7 0 0 1 0-1.3l3.2-.8.8-3.2z"/>' +
      '<path fill="currentColor" d="M6.4 15.6a.5.5 0 0 1 .4 0l.5 2.1 2.1.5a.5.5 0 0 1 0 .9l-2.1.5-.5 2.1a.5.5 0 0 1-.9 0l-.5-2.1-2.1-.5a.5.5 0 0 1 0-.9l2.1-.5.5-2.1z"/>',
    panelWidth: 340,
    panelHeight: 460,
  },
  {
    key: 'plan-task',
    title: 'Todos',
    fabClass: 'pt-assistant-fab',
    fabIconClass: 'pt-assistant-fab-icon',
    iconPaths:
      '<path fill="currentColor" d="M4 5.5A1.5 1.5 0 0 1 5.5 4h13A1.5 1.5 0 0 1 20 5.5v13a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 4 18.5v-13zM7 8h10v1.5H7V8zm0 3.5h10V13H7v-1.5zm0 3.5h6V16H7v-1z"/>',
    panelWidth: 340,
    panelHeight: 460,
  },
  {
    key: 'notes',
    title: 'Notes Assistant',
    fabClass: 'note-assistant-fab',
    fabIconClass: 'note-assistant-fab-icon',
    iconPaths:
      '<path fill="currentColor" d="M6 3.5A1.5 1.5 0 0 0 4.5 5v14A1.5 1.5 0 0 0 6 20.5h9.5a.75.75 0 0 0 .53-.22l3.25-3.25a.75.75 0 0 0 .22-.53V5A1.5 1.5 0 0 0 18 3.5H6zm8.75 13.25V19H6.5V5.5h11v9.75H15.5a.75.75 0 0 0-.75.75z"/>',
    panelWidth: 340,
    panelHeight: 460,
  },
  {
    key: 'builders',
    title: 'Builders',
    fabClass: 'builders-entry-fab',
    fabIconClass: 'builders-entry-fab-icon',
    iconPaths:
      '<path fill="currentColor" d="M4.5 5.75A1.75 1.75 0 0 1 6.25 4h11.5A1.75 1.75 0 0 1 19.5 5.75v12.5A1.75 1.75 0 0 1 17.75 20H6.25A1.75 1.75 0 0 1 4.5 18.25V5.75zm2 1.5v2.5h11v-2.5h-11zm0 4.5v2.5h7.5v-2.5h-7.5zm0 4.5v2.5h5v-2.5h-5z"/>',
    panelWidth: 480,
    panelHeight: 520,
  },
];

/** Default panel size when an entry omits width/height. */
export const DEFAULT_PANEL = Object.freeze({ width: 340, height: 460 });

/**
 * AI bypass entry — pinned beside hub, not part of hub-expand baseline.
 * @returns {EntryConfig}
 */
export function getAiAssistantEntry() {
  return {
    id: 'ai-assistant',
    contentKey: 'ai-assistant',
    title: 'Assistant',
    overlayTitle: 'Assistant',
    fabClass: 'ai-assistant-fab',
    fabIconClass: 'ai-assistant-fab-icon',
    iconPaths:
      '<path fill="currentColor" d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20zm1 5v4h4v2h-4v4h-2v-4H7v-2h4V7h2z"/>',
    panelWidth: 340,
    panelHeight: 460,
  };
}

/** @returns {EntryConfig[]} */
export function getBaselineEntries() {
  return BASELINE.map(({ key, title, fabClass, fabIconClass, iconPaths, panelWidth, panelHeight }) => ({
    id: key,
    contentKey: key,
    title,
    overlayTitle: title,
    fabClass,
    fabIconClass,
    iconPaths,
    panelWidth,
    panelHeight,
  }));
}

/** @returns {EntryConfig[]} */
export function createEntryConfig() {
  return getBaselineEntries();
}
