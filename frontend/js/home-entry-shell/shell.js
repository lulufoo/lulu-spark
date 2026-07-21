/**
 * Home-entry shell UI: entry cluster + OverlayChrome + content slot.
 * Dispatches DOM events into the A/B/C FSM; no business UI inside the shell.
 */

import { createHomeEntryFsm } from './fsm.js';

/**
 * @typedef {{ id: string, contentKey: string, title: string, overlayTitle: string }} EntryConfig
 * @typedef {{ register: Function, get: (contentKey: string) => unknown }} ContentRegistry
 * @typedef {{ mount?: (slot: HTMLElement, ctx: { entry: EntryConfig, host: unknown }) => (void | { unmount?: () => void }) }} ContentAdapter
 */

/**
 * @param {HTMLElement} anchor
 * @param {{ config: EntryConfig[], registry: ContentRegistry, host?: unknown, fsm?: ReturnType<typeof createHomeEntryFsm> }} opts
 * @returns {{ unmount: () => void, getState: () => import('./fsm.js').FsmSnapshot | { mode: string, entryId?: string } }}
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

  function clearContent() {
    if (activeContent && typeof activeContent.unmount === 'function') {
      activeContent.unmount();
    }
    activeContent = null;
    activeEntryId = null;
    slotEl.replaceChildren();
  }

  /**
   * @param {EntryConfig} entry
   */
  function mountContent(entry) {
    clearContent();
    titleEl.textContent = entry.overlayTitle || entry.title;
    /** @type {ContentAdapter | undefined} */
    const adapter = /** @type {ContentAdapter | undefined} */ (registry.get(entry.contentKey));
    if (adapter && typeof adapter.mount === 'function') {
      const handle = adapter.mount(slotEl, { entry, host });
      activeContent = handle && typeof handle === 'object' ? handle : null;
      activeEntryId = entry.id;
    }
  }

  function syncDom() {
    const snap = machine.snapshot();
    const mode = snap.mode;
    root.dataset.state = mode;
    hubBtn.setAttribute('aria-expanded', String(mode !== 'A'));

    // A: hub only; B/C: business entries visible (cluster stays expanded in C).
    entriesWrap.hidden = mode === 'A';
    overlay.hidden = mode !== 'C';

    if (mode === 'C') {
      const entry = entries.find((e) => e.id === snap.entryId);
      if (entry && activeEntryId !== entry.id) {
        mountContent(entry);
      }
    } else if (activeEntryId != null) {
      clearContent();
      titleEl.textContent = '';
    }
  }

  function getState() {
    return machine.snapshot();
  }

  hubBtn.addEventListener('click', (event) => {
    event.stopPropagation();
    const mode = machine.getState();
    if (mode === 'A') {
      machine.dispatch({ type: 'openHub' });
    } else if (mode === 'B') {
      machine.dispatch({ type: 'closeHub' });
    }
    // C: hub click must keep C (FSM rejects closeHub).
    syncDom();
  });

  entriesWrap.addEventListener('click', (event) => {
    const target = /** @type {HTMLElement | null} */ (event.target);
    const btn = target?.closest?.('[data-role="entry"]');
    if (!btn || !entriesWrap.contains(btn)) return;
    event.stopPropagation();
    const entryId = btn.getAttribute('data-entry-id');
    if (!entryId) return;
    machine.dispatch({ type: 'openEntry', entryId });
    syncDom();
  });

  closeEl.addEventListener('click', (event) => {
    event.stopPropagation();
    machine.dispatch({ type: 'closeOverlay' });
    syncDom();
  });

  backdrop.addEventListener('click', (event) => {
    event.stopPropagation();
    machine.dispatch({ type: 'closeOverlay' });
    syncDom();
  });

  /**
   * @param {MouseEvent} event
   */
  function onDocClick(event) {
    const mode = machine.getState();
    if (mode !== 'B' && mode !== 'C') return;
    const target = /** @type {Node | null} */ (event.target);
    if (target && cluster.contains(target)) return;
    if (mode === 'C') {
      // Overlay chrome clicks (except backdrop, handled above) should not collapse via outside.
      if (target && chrome.contains(target)) return;
      machine.dispatch({ type: 'closeOverlay' });
      syncDom();
      return;
    }
    // B: outside cluster → B→A
    machine.dispatch({ type: 'closeHub' });
    syncDom();
  }

  /**
   * @param {KeyboardEvent} event
   */
  function onKeyDown(event) {
    if (event.key !== 'Escape') return;
    if (machine.getState() !== 'C') return;
    machine.dispatch({ type: 'closeOverlay' });
    syncDom();
  }

  document.addEventListener('click', onDocClick, true);
  document.addEventListener('keydown', onKeyDown, true);

  syncDom();

  function unmount() {
    document.removeEventListener('click', onDocClick, true);
    document.removeEventListener('keydown', onKeyDown, true);
    clearContent();
    root.remove();
  }

  return { unmount, getState };
}
