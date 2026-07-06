import { fetchKbDocCount } from './api.js';
import { parseHash } from './router/index.js';

export const BADGE_BASE_TITLE = '⚙ 沉淀知识库';

const DEBOUNCE_MS = 300;
const FETCH_TIMEOUT_MS = 10_000;

let debounceTimer = null;
let requestSeq = 0;
let lastFetchedRepo = null;
let listenersAttached = false;
/** @type {Promise<void> | null} */
let inFlightRefresh = null;
/** @type {string | null} */
let inFlightRefreshRepo = null;
let lastHandledHash = null;

/** @param {string | undefined} [hash] */
export function getCorpusRepoFromHash(hash) {
  const route = parseHash(hash);
  if (route.name !== 'corpus-doc') return null;
  const repo = route.params.repo;
  return repo ? repo : null;
}

/** @param {number | null} count */
export function formatBadgeTitle(count) {
  if (count == null || Number.isNaN(count)) {
    return BADGE_BASE_TITLE;
  }
  return `${BADGE_BASE_TITLE} · ${Math.floor(count)}`;
}

function applyBadgeTitle(count) {
  const btn = document.getElementById('btn-repo-menu');
  if (!btn) return;
  btn.textContent = formatBadgeTitle(count);
}

async function refreshBadge() {
  const repo = getCorpusRepoFromHash();
  if (!repo) {
    inFlightRefresh = null;
    inFlightRefreshRepo = null;
    lastFetchedRepo = null;
    applyBadgeTitle(null);
    return;
  }

  if (inFlightRefresh && inFlightRefreshRepo === repo) {
    return inFlightRefresh;
  }

  const seq = ++requestSeq;
  if (lastFetchedRepo !== repo) {
    applyBadgeTitle(null);
  }
  lastFetchedRepo = repo;

  inFlightRefreshRepo = repo;
  inFlightRefresh = (async () => {
    let timeoutId;
    try {
      const count = await Promise.race([
        fetchKbDocCount(repo),
        new Promise((_, reject) => {
          timeoutId = setTimeout(() => reject(new Error('timeout')), FETCH_TIMEOUT_MS);
        }),
      ]);
      if (getCorpusRepoFromHash() !== repo || seq !== requestSeq) return;
      applyBadgeTitle(count);
    } catch {
      if (getCorpusRepoFromHash() !== repo || seq !== requestSeq) return;
      applyBadgeTitle(null);
    } finally {
      clearTimeout(timeoutId);
      if (inFlightRefreshRepo === repo) {
        inFlightRefresh = null;
        inFlightRefreshRepo = null;
      }
    }
  })();

  return inFlightRefresh;
}

function scheduleDebouncedRefresh() {
  clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    void refreshBadge();
  }, DEBOUNCE_MS);
}

function onHashChange() {
  const hash = window.location.hash;
  if (hash === lastHandledHash) return;
  lastHandledHash = hash;
  clearTimeout(debounceTimer);
  void refreshBadge();
}

export function initSedimentKbBadge() {
  if (listenersAttached) {
    void refreshBadge();
    return;
  }
  listenersAttached = true;
  lastHandledHash = window.location.hash;
  window.addEventListener('hashchange', onHashChange);
  window.addEventListener('kb:hide-pattern-changed', scheduleDebouncedRefresh);
  window.addEventListener('kb-diff-updated', scheduleDebouncedRefresh);
  void refreshBadge();
}
