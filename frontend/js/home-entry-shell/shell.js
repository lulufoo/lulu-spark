/**
 * Home-entry shell UI: entry cluster + OverlayChrome + content slot.
 * Dispatches DOM events into the A/B/C FSM; no business UI inside the shell.
 */

import { createHomeEntryFsm } from './fsm.js';

/**
 * @typedef {{ id: string, contentKey: string, title: string, overlayTitle: string }} EntryConfig
 * @typedef {{ register: Function, get: (contentKey: string) => unknown }} ContentRegistry
 * @typedef {{ mount?: (slot: HTMLElement, ctx: { entry: EntryConfig, host: unknown }) => (void | { unmount?: () => void } | Promise<void | { unmount?: () => void }>) }} ContentAdapter
 */

/**
 * @param {HTMLElement} anchor
 * @param {{ config: EntryConfig[], registry: ContentRegistry, host?: unknown, fsm?: ReturnType<typeof createHomeEntryFsm> }} opts
 * @returns {{
 *   unmount: () => void,
 *   getState: () => ReturnType<ReturnType<typeof createHomeEntryFsm>['snapshot']>,
 *   openContent: (entryId: string) => Promise<void>,
 *   forceRecoverA: (reason?: string) => void,
 * }}
 */
export function mountHomeEntryShell(anchor, { config, registry, host = {}, fsm } = {}) {
  const machine = fsm ?? createHomeEntryFsm();
  const entries = Array.isArray(config) ? config : [];

  const root = document.createElement('div');
  root.className = 'home-entry-shell';
  root.dataset.state = 'A';

  const cluster = document.createElement('div');
  cluster.className = 'home-entry-shell__cluster';
  cluster.dataset.role = 'cluster';

  const hubBtn = document.createElement('button');
  hubBtn.type = 'button';
  hubBtn.className = 'home-entry-shell__hub';
  hubBtn.dataset.role = 'hub';
  hubBtn.setAttribute('aria-label', 'Open entry hub');
  hubBtn.setAttribute('aria-expanded', 'false');
  hubBtn.title = 'Entries';
  hubBtn.textContent = '☰';

  const entriesWrap = document.createElement('div');
  entriesWrap.className = 'home-entry-shell__entries';
  entriesWrap.dataset.role = 'entries';
  entriesWrap.hidden = true;

  for (const entry of entries) {
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'home-entry-shell__entry';
    btn.dataset.role = 'entry';
    btn.dataset.entryId = entry.id;
    btn.title = entry.title;
    btn.setAttribute('aria-label', `Open ${entry.title}`);
    btn.textContent = entry.title;
    entriesWrap.appendChild(btn);
  }

  // DOM order: entries above hub visually via column-reverse stack.
  cluster.appendChild(entriesWrap);
  cluster.appendChild(hubBtn);

  const overlay = document.createElement('div');
  overlay.className = 'home-entry-shell__overlay';
  overlay.dataset.role = 'overlay';
  overlay.hidden = true;

  const backdrop = document.createElement('div');
  backdrop.className = 'home-entry-shell__backdrop';
  backdrop.dataset.role = 'backdrop';

  const chrome = document.createElement('div');
  chrome.className = 'home-entry-shell__chrome';
  chrome.dataset.role = 'chrome';
  chrome.innerHTML = `
    <header class="home-entry-shell__header">
      <span class="home-entry-shell__title" data-role="title"></span>
      <button type="button" class="home-entry-shell__close" data-role="close" aria-label="Close">×</button>
    </header>
    <div class="home-entry-shell__slot" data-role="content-slot"></div>
  `;

  overlay.appendChild(backdrop);
  overlay.appendChild(chrome);
  root.appendChild(cluster);
  root.appendChild(overlay);
  anchor.appendChild(root);

  const titleEl = /** @type {HTMLElement} */ (chrome.querySelector('[data-role="title"]'));
  const closeEl = /** @type {HTMLElement} */ (chrome.querySelector('[data-role="close"]'));
  const slotEl = /** @type {HTMLElement} */ (chrome.querySelector('[data-role="content-slot"]'));

  /** @type {{ unmount?: () => void } | null} */
  let activeContent = null;
  /** @type {string | null} */
  let activeEntryId = null;
  /** @type {{ entryId: string } | null} */
  let failurePresentation = null;
  let recovering = false;

  function clearSlotDom() {
    slotEl.replaceChildren();
  }

  /**
   * Detach active content handle.
   * @returns {'ok' | 'error'}
   */
  function detachActiveContent() {
    const prev = activeContent;
    activeContent = null;
    activeEntryId = null;
    try {
      if (prev && typeof prev.unmount === 'function') {
        prev.unmount();
      }
      return 'ok';
    } catch {
      return 'error';
    }
  }

  /**
   * Unmount active content. On unmount throw → force A (close/exception recovery).
   * @returns {boolean} true if recovery was triggered
   */
  function clearContent() {
    const result = detachActiveContent();
    clearSlotDom();
    if (result === 'error') {
      titleEl.textContent = '';
      failurePresentation = null;
      forceRecoverA('close-failed');
      return true;
    }
    return false;
  }

  /** @param {EntryConfig} entry */
  function presentFailure(entry) {
    if (machine.getState() === 'C') {
      // Leave success C without treating content unmount as close-failure recovery.
      detachActiveContent();
      clearSlotDom();
      machine.dispatch({ type: 'closeOverlay' });
    }

    failurePresentation = { entryId: entry.id };
    titleEl.textContent = entry.overlayTitle || entry.title;
    clearSlotDom();
    const err = document.createElement('div');
    err.className = 'home-entry-shell__content-error';
    err.dataset.role = 'content-error';
    err.textContent = 'Failed to load content';
    slotEl.appendChild(err);
    syncDom();
  }

  /** @param {string} [reason] */
  function forceRecoverA(reason) {
    if (recovering) return;
    recovering = true;
    try {
      failurePresentation = null;
      detachActiveContent();
      clearSlotDom();
      titleEl.textContent = '';
      machine.dispatch({ type: 'forceA' });
      syncDom();
    } finally {
      recovering = false;
    }
  }

  /** @param {EntryConfig} entry */
  function mountContent(entry) {
    if (clearContent()) return;
    failurePresentation = null;
    titleEl.textContent = entry.overlayTitle || entry.title;
    const adapter = /** @type {ContentAdapter | undefined} */ (registry.get(entry.contentKey));
    if (adapter && typeof adapter.mount === 'function') {
      const handle = adapter.mount(slotEl, { entry, host });
      activeContent = handle && typeof handle === 'object' && !('then' in /** @type {object} */ (handle))
        ? /** @type {{ unmount?: () => void }} */ (handle)
        : null;
      activeEntryId = entry.id;
    }
  }

  function syncDom() {
    if (recovering) {
      // forceRecoverA owns DOM reset; only refresh chrome visibility from snapshot.
      const snap = machine.snapshot();
      root.dataset.state = snap.mode;
      hubBtn.setAttribute('aria-expanded', String(snap.mode !== 'A'));
      entriesWrap.hidden = snap.mode === 'A';
      overlay.hidden = true;
      return;
    }

    const snap = machine.snapshot();
    const mode = snap.mode;
    root.dataset.state = mode;
    hubBtn.setAttribute('aria-expanded', String(mode !== 'A'));
    // A: hub only; B/C: entries visible (cluster stays expanded while overlay is open).
    entriesWrap.hidden = mode === 'A';
    overlay.hidden = !(mode === 'C' || failurePresentation != null);

    if (mode === 'A') {
      failurePresentation = null;
      if (activeEntryId != null || slotEl.childNodes.length > 0 || titleEl.textContent) {
        if (clearContent()) return;
        titleEl.textContent = '';
      }
      return;
    }

    if (mode === 'C') {
      failurePresentation = null;
      const entry = entries.find((e) => e.id === snap.entryId);
      if (entry && activeEntryId !== entry.id) {
        mountContent(entry);
      }
      return;
    }

    // B: keep failure end-state; otherwise clear leftover success content.
    if (!failurePresentation && activeEntryId != null) {
      if (clearContent()) return;
      titleEl.textContent = '';
    }
  }

  /** @param {import('./fsm.js').FsmEvent | { type: string, entryId?: string }} event */
  function dispatchAndSync(event) {
    machine.dispatch(/** @type {any} */ (event));
    syncDom();
  }

  /**
   * Load content via registry; success → C; missing key / load failure → stay B with error placeholder.
   * @param {string} entryId
   * @returns {Promise<void>}
   */
  async function openContent(entryId) {
    const entry = entries.find((e) => e.id === entryId);
    if (!entry) return;
    if (machine.getState() === 'A') return;

    const adapter = /** @type {ContentAdapter | undefined} */ (registry.get(entry.contentKey));
    if (!adapter || typeof adapter.mount !== 'function') {
      presentFailure(entry);
      return;
    }

    // Drop prior success/failure presentation before attempting a new mount.
    failurePresentation = null;
    if (activeEntryId != null || activeContent) {
      if (detachActiveContent() === 'error') {
        clearSlotDom();
        forceRecoverA('exception');
        return;
      }
      clearSlotDom();
    }

    titleEl.textContent = entry.overlayTitle || entry.title;

    try {
      const mounted = adapter.mount(slotEl, { entry, host });
      const handle = mounted && typeof mounted === 'object' && typeof /** @type {{ then?: unknown }} */ (mounted).then === 'function'
        ? await /** @type {Promise<void | { unmount?: () => void }>} */ (mounted)
        : mounted;

      activeContent = handle && typeof handle === 'object' ? /** @type {{ unmount?: () => void }} */ (handle) : null;
      activeEntryId = entry.id;
      failurePresentation = null;
      machine.dispatch({ type: 'openEntry', entryId });
      syncDom();
    } catch {
      activeContent = null;
      activeEntryId = null;
      presentFailure(entry);
    }
  }

  hubBtn.addEventListener('click', (event) => {
    event.stopPropagation();
    const mode = machine.getState();
    if (mode === 'A') {
      dispatchAndSync({ type: 'openHub' });
    } else if (mode === 'B') {
      failurePresentation = null;
      dispatchAndSync({ type: 'closeHub' });
    }
    // C: hub click keeps C (FSM rejects closeHub); no dispatch needed.
  });

  entriesWrap.addEventListener('click', (event) => {
    const target = /** @type {HTMLElement | null} */ (event.target);
    const btn = target?.closest?.('[data-role="entry"]');
    if (!btn || !entriesWrap.contains(btn)) return;
    event.stopPropagation();
    const entryId = btn.getAttribute('data-entry-id');
    if (!entryId) return;
    void openContent(entryId);
  });

  function closeOverlay(event) {
    event.stopPropagation();
    if (failurePresentation && machine.getState() !== 'C') {
      failurePresentation = null;
      clearSlotDom();
      titleEl.textContent = '';
      syncDom();
      return;
    }
    if (clearContent()) return;
    failurePresentation = null;
    titleEl.textContent = '';
    dispatchAndSync({ type: 'closeOverlay' });
  }

  closeEl.addEventListener('click', closeOverlay);
  backdrop.addEventListener('click', closeOverlay);

  /** @param {MouseEvent} event */
  function onDocClick(event) {
    const mode = machine.getState();
    if (mode !== 'B' && mode !== 'C') return;
    const target = /** @type {Node | null} */ (event.target);
    if (target && cluster.contains(target)) return;
    if (mode === 'C') {
      // Chrome clicks stay open; backdrop/outside close via C→B.
      if (target && chrome.contains(target)) return;
      if (clearContent()) return;
      failurePresentation = null;
      titleEl.textContent = '';
      dispatchAndSync({ type: 'closeOverlay' });
      return;
    }
    failurePresentation = null;
    dispatchAndSync({ type: 'closeHub' });
  }

  /** @param {KeyboardEvent} event */
  function onKeyDown(event) {
    if (event.key !== 'Escape') return;
    if (failurePresentation && machine.getState() === 'B') {
      failurePresentation = null;
      clearSlotDom();
      titleEl.textContent = '';
      syncDom();
      return;
    }
    if (machine.getState() !== 'C') return;
    if (clearContent()) return;
    failurePresentation = null;
    titleEl.textContent = '';
    dispatchAndSync({ type: 'closeOverlay' });
  }

  document.addEventListener('click', onDocClick, true);
  document.addEventListener('keydown', onKeyDown, true);
  syncDom();

  function unmount() {
    document.removeEventListener('click', onDocClick, true);
    document.removeEventListener('keydown', onKeyDown, true);
    // Host teardown / leave: force A so overlay state does not survive.
    forceRecoverA('unmount');
    root.remove();
  }

  return {
    unmount,
    getState: () => machine.snapshot(),
    openContent,
    forceRecoverA,
  };
}
