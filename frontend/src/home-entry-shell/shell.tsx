/**
 * Home-entry shell UI: entry cluster + OverlayChrome + content slot.
 * Dispatches DOM events into the A/B/C FSM; no business UI inside the shell.
 */

import { flushSync } from 'react-dom';
import { createRoot, type Root } from 'react-dom/client';
import { createHomeEntryFsm } from './fsm.ts';
import { DEFAULT_PANEL } from './entry-config.ts';

type EntryConfig = {
  id: string;
  contentKey: string;
  title: string;
  overlayTitle: string;
  fabClass?: string;
  fabIconClass?: string;
  iconPaths?: string;
  panelWidth?: number;
  panelHeight?: number;
};

type ContentRegistry = { register: Function; get: (contentKey: string) => unknown };
type ContentAdapter = {
  mount?: (
    slot: HTMLElement,
    ctx: { entry: EntryConfig; host: unknown },
  ) => void | { unmount?: () => void } | Promise<void | { unmount?: () => void }>;
};

function ShellView({ entries }: { entries: EntryConfig[] }) {
  return (
    <>
      <div className="home-entry-shell__cluster" data-role="cluster">
        <div className="home-entry-shell__entries" data-role="entries" aria-hidden="true">
          {entries.map((entry) => {
            const fabClass = entry.fabClass || 'home-entry-shell__entry-fab';
            const iconClass = entry.fabIconClass || 'home-entry-shell__entry-icon';
            return (
              <button
                key={entry.id}
                type="button"
                className={`home-entry-shell__entry ${fabClass}`}
                data-role="entry"
                data-entry-id={entry.id}
                data-fab-class={entry.fabClass}
                title={entry.title}
                aria-label={`Open ${entry.title}`}
              >
                {entry.iconPaths ? (
                  <svg
                    className={iconClass}
                    viewBox="0 0 24 24"
                    aria-hidden="true"
                    focusable="false"
                    dangerouslySetInnerHTML={{ __html: entry.iconPaths }}
                  />
                ) : (
                  entry.title
                )}
              </button>
            );
          })}
        </div>
        <button
          type="button"
          className="home-entry-shell__hub"
          data-role="hub"
          aria-label="Open entry hub"
          aria-expanded="false"
          title="Entries"
        >
          <svg className="home-entry-shell__hub-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <path
              fill="currentColor"
              d="M11 5a1 1 0 1 1 2 0v6h6a1 1 0 1 1 0 2h-6v6a1 1 0 1 1-2 0v-6H5a1 1 0 1 1 0-2h6V5z"
            />
          </svg>
        </button>
      </div>
      <div className="home-entry-shell__overlay" data-role="overlay" hidden>
        <div className="home-entry-shell__backdrop" data-role="backdrop" />
        <div className="home-entry-shell__chrome" data-role="chrome">
          <header className="home-entry-shell__header">
            <span className="home-entry-shell__title" data-role="title" />
            <button type="button" className="home-entry-shell__close" data-role="close" aria-label="Close">
              ×
            </button>
          </header>
          <div className="home-entry-shell__slot" data-role="content-slot" />
        </div>
      </div>
    </>
  );
}

export function mountHomeEntryShell(
  anchor: HTMLElement,
  {
    config,
    registry,
    host = {},
    fsm,
  }: {
    config?: EntryConfig[];
    registry: ContentRegistry;
    host?: unknown;
    fsm?: ReturnType<typeof createHomeEntryFsm>;
  } = { registry: { register() {}, get() { return undefined; } } },
) {
  const machine = fsm ?? createHomeEntryFsm();
  const entries = Array.isArray(config) ? config : [];

  const root = document.createElement('div');
  root.className = 'home-entry-shell';
  root.dataset.state = 'A';
  anchor.appendChild(root);
  const reactRoot: Root = createRoot(root);
  flushSync(() => {
    reactRoot.render(<ShellView entries={entries} />);
  });

  const cluster = root.querySelector('[data-role="cluster"]') as HTMLElement;
  const hubBtn = root.querySelector('[data-role="hub"]') as HTMLElement;
  const entriesWrap = root.querySelector('[data-role="entries"]') as HTMLElement;
  const overlay = root.querySelector('[data-role="overlay"]') as HTMLElement;
  const backdrop = root.querySelector('[data-role="backdrop"]') as HTMLElement;
  const chrome = root.querySelector('[data-role="chrome"]') as HTMLElement;
  const titleEl = chrome.querySelector('[data-role="title"]') as HTMLElement;
  const closeEl = chrome.querySelector('[data-role="close"]') as HTMLElement;
  const slotEl = chrome.querySelector('[data-role="content-slot"]') as HTMLElement;

  let activeContent: { unmount?: () => void } | null = null;
  let activeEntryId: string | null = null;
  let failurePresentation: { entryId: string } | null = null;
  let recovering = false;

  function clearSlotDom() {
    slotEl.replaceChildren();
  }

  function applyPanelSize(entry: EntryConfig | null | undefined) {
    const width = Number(entry?.panelWidth) > 0 ? Number(entry?.panelWidth) : DEFAULT_PANEL.width;
    const height = Number(entry?.panelHeight) > 0 ? Number(entry?.panelHeight) : DEFAULT_PANEL.height;
    chrome.style.setProperty('--hes-panel-w', `${width}px`);
    chrome.style.setProperty('--hes-panel-h', `${height}px`);
  }

  function syncEntryActive(activeId: string | null) {
    for (const btn of entriesWrap.querySelectorAll('[data-role="entry"]')) {
      const entryId = btn.getAttribute('data-entry-id');
      const fabClass = btn.getAttribute('data-fab-class');
      const isActive = Boolean(activeId && entryId === activeId);
      btn.classList.toggle('home-entry-shell__entry--active', isActive);
      if (fabClass) {
        btn.classList.toggle(`${fabClass}--active`, isActive);
      }
      btn.setAttribute('aria-expanded', String(isActive));
    }
  }

  function detachActiveContent(): 'ok' | 'error' {
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

  function presentFailure(entry: EntryConfig) {
    if (machine.getState() === 'C') {
      detachActiveContent();
      clearSlotDom();
      machine.dispatch({ type: 'closeOverlay' });
    }

    failurePresentation = { entryId: entry.id };
    applyPanelSize(entry);
    titleEl.textContent = entry.overlayTitle || entry.title;
    clearSlotDom();
    const err = document.createElement('div');
    err.className = 'home-entry-shell__content-error';
    err.dataset.role = 'content-error';
    err.textContent = 'Failed to load content';
    slotEl.appendChild(err);
    syncDom();
  }

  function forceRecoverA(_reason?: string) {
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

  function mountContent(entry: EntryConfig) {
    if (clearContent()) return;
    failurePresentation = null;
    applyPanelSize(entry);
    titleEl.textContent = entry.overlayTitle || entry.title;
    const adapter = registry.get(entry.contentKey) as ContentAdapter | undefined;
    if (adapter && typeof adapter.mount === 'function') {
      const handle = adapter.mount(slotEl, { entry, host });
      activeContent =
        handle && typeof handle === 'object' && !('then' in handle)
          ? (handle as { unmount?: () => void })
          : null;
      activeEntryId = entry.id;
    }
  }

  function findEntry(entryId: string | undefined) {
    return entries.find((e) => e.id === entryId);
  }

  function syncDom() {
    if (recovering) {
      const snap = machine.snapshot();
      root.dataset.state = snap.mode;
      root.dataset.entryId = snap.mode === 'C' ? snap.entryId : '';
      hubBtn.setAttribute('aria-expanded', String(snap.mode !== 'A'));
      entriesWrap.setAttribute('aria-hidden', String(snap.mode !== 'B'));
      syncEntryActive(null);
      overlay.hidden = true;
      return;
    }

    const snap = machine.snapshot();
    const mode = snap.mode;
    root.dataset.state = mode;
    root.dataset.entryId = mode === 'C' ? snap.entryId ?? '' : '';
    hubBtn.setAttribute('aria-expanded', String(mode !== 'A'));
    entriesWrap.setAttribute('aria-hidden', String(mode === 'A'));
    overlay.hidden = !(mode === 'C' || failurePresentation != null);

    if (mode === 'A') {
      failurePresentation = null;
      syncEntryActive(null);
      if (activeEntryId != null || slotEl.childNodes.length > 0 || titleEl.textContent) {
        if (clearContent()) return;
        titleEl.textContent = '';
      }
      return;
    }

    if (mode === 'C') {
      failurePresentation = null;
      const entry = findEntry(snap.entryId);
      if (entry && activeEntryId !== entry.id) {
        mountContent(entry);
      } else if (entry) {
        applyPanelSize(entry);
      }
      syncEntryActive(snap.entryId ?? null);
      return;
    }

    if (failurePresentation) {
      const failed = findEntry(failurePresentation.entryId);
      if (failed) applyPanelSize(failed);
      syncEntryActive(failurePresentation.entryId);
    } else {
      syncEntryActive(null);
      if (activeEntryId != null) {
        if (clearContent()) return;
        titleEl.textContent = '';
      }
    }
  }

  function dispatchAndSync(event: { type: string; entryId?: string }) {
    machine.dispatch(event as never);
    syncDom();
  }

  async function openContent(entryId: string) {
    const entry = findEntry(entryId);
    if (!entry) return;
    const mode = machine.getState();
    if (mode === 'A') return;

    const adapter = registry.get(entry.contentKey) as ContentAdapter | undefined;
    if (!adapter || typeof adapter.mount !== 'function') {
      presentFailure(entry);
      return;
    }

    failurePresentation = null;
    if (activeEntryId != null || activeContent) {
      if (detachActiveContent() === 'error') {
        clearSlotDom();
        forceRecoverA('exception');
        return;
      }
      clearSlotDom();
    }

    applyPanelSize(entry);
    titleEl.textContent = entry.overlayTitle || entry.title;

    try {
      const mounted = adapter.mount(slotEl, { entry, host });
      const handle =
        mounted && typeof mounted === 'object' && typeof (mounted as { then?: unknown }).then === 'function'
          ? await (mounted as Promise<void | { unmount?: () => void }>)
          : mounted;

      activeContent = handle && typeof handle === 'object' ? (handle as { unmount?: () => void }) : null;
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
  });

  entriesWrap.addEventListener('click', (event) => {
    const target = event.target as HTMLElement | null;
    const btn = target?.closest?.('[data-role="entry"]');
    if (!btn || !entriesWrap.contains(btn)) return;
    event.stopPropagation();
    const entryId = btn.getAttribute('data-entry-id');
    if (!entryId) return;
    void openContent(entryId);
  });

  function closeOverlay(event: Event) {
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

  function onDocClick(event: MouseEvent) {
    const mode = machine.getState();
    if (mode !== 'B' && mode !== 'C') return;
    const target = event.target as Node | null;
    if (target && cluster.contains(target)) return;
    if (mode === 'C') {
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

  function onKeyDown(event: KeyboardEvent) {
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
    forceRecoverA('unmount');
    flushSync(() => {
      reactRoot.unmount();
    });
    root.remove();
  }

  return {
    unmount,
    getState: () => machine.snapshot(),
    openContent,
    forceRecoverA,
  };
}
