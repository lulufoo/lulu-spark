import {
  bindFocusRefresh,
  loadReadLaterEntries,
  markEntryRead,
  openExternalUrl,
} from './list.js';
import { escHtml } from '../shared/utils.js';

export { bindFocusRefresh };

const UNAVAILABLE_MSG = 'List temporarily unavailable. Please try again later.';

export async function loadAssistantEntries() {
  return loadReadLaterEntries();
}

export function selectTop3Latest(entries) {
  return [...entries]
    .sort((a, b) => {
      const aTime = Date.parse(a.saved_at ?? '') || 0;
      const bTime = Date.parse(b.saved_at ?? '') || 0;
      return bTime - aTime;
    })
    .slice(0, 3);
}

function formatSavedAt(savedAt) {
  if (!savedAt) return '';
  const date = new Date(savedAt);
  if (Number.isNaN(date.getTime())) return savedAt;
  return date.toLocaleString('en', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

function renderEmpty() {
  return `
    <div class="read-later-assistant-empty read-later-assistant-state">
      <p class="read-later-assistant-state-title">No items to read later</p>
      <p class="read-later-assistant-state-detail">After saving with the Chrome extension, latest items appear here</p>
    </div>
  `;
}

function renderUnavailable(message = UNAVAILABLE_MSG) {
  return `<div class="read-later-assistant-unavailable read-later-assistant-banner">${escHtml(message)}</div>`;
}

function renderErrorEmpty(message = UNAVAILABLE_MSG) {
  return `
    <div class="read-later-assistant-empty read-later-assistant-state read-later-assistant-state--error">
      <p class="read-later-assistant-state-title">Temporarily unavailable</p>
      <p class="read-later-assistant-state-detail">${escHtml(message)}</p>
    </div>
  `;
}

function entryReadClass(baseClass, read) {
  const stateClass = read
    ? 'read-later-assistant-current--read'
    : 'read-later-assistant-current--unread';
  return `${baseClass} ${stateClass}`;
}

function renderPanel(entries, selectedIndex) {
  const current = entries[selectedIndex];
  const showCycle = entries.length > 1;
  const pickerHtml = showCycle
    ? `<div class="read-later-assistant-picker" role="tablist" aria-label="Recent items">${entries
        .map((entry, index) => {
          const activeClass =
            index === selectedIndex ? ' read-later-assistant-picker-item--active' : '';
          const readClass = entry.read
            ? ' read-later-assistant-picker-item--read'
            : ' read-later-assistant-picker-item--unread';
          const label = entry.title || entry.url;
          return `<button type="button" class="read-later-assistant-picker-item${readClass}${activeClass}" data-entry-id="${escHtml(entry.id)}" data-index="${index}" title="${escHtml(label)}"><span class="read-later-assistant-picker-rank">${index + 1}</span><span class="read-later-assistant-picker-label">${escHtml(label)}</span></button>`;
        })
        .join('')}</div>`
    : '';

  const url = escHtml(current.url);
  const title = escHtml(current.title || current.url);
  const cycleHtml = showCycle
    ? '<button type="button" class="read-later-assistant-cycle" aria-label="Next">›</button>'
    : '';
  const currentClass = entryReadClass(
    showCycle
      ? 'read-later-assistant-current read-later-assistant-current--has-cycle'
      : 'read-later-assistant-current',
    current.read,
  );

  return `
    <div class="read-later-assistant-panel">
      ${pickerHtml}
      <article class="${currentClass}">
        <div class="read-later-assistant-current-main">
          <a class="read-later-assistant-entry-link" href="${url}" data-url="${url}" data-entry-id="${escHtml(current.id)}">
            <h3 class="read-later-assistant-current-title">${title}</h3>
            <p class="read-later-assistant-current-url" title="${url}">${url}</p>
          </a>
          <p class="read-later-assistant-current-saved-at">${escHtml(formatSavedAt(current.saved_at))}</p>
        </div>
        ${cycleHtml}
      </article>
    </div>
  `;
}

function renderManageLink(showManage) {
  if (!showManage) return '';
  return `<button type="button" class="read-later-assistant-manage-link">View all read-later →</button>`;
}

/**
 * @param {HTMLElement} root
 * @param {{ autoLoad?: boolean, navigate?: (hash: string) => void, openReadLater?: () => void }} [opts]
 */
export function mountReadLaterAssistant(root, opts = {}) {
  const { autoLoad = true, navigate, openReadLater } = opts;
  let disposed = false;
  let top3Entries = [];
  let selectedIndex = 0;
  let lastSuccessfulTop3 = null;
  let refreshPromise = null;

  const showManage = typeof openReadLater === 'function' || typeof navigate === 'function';

  function renderCurrent({ showUnavailable = false } = {}) {
    const footer = renderManageLink(showManage);
    if (!top3Entries.length) {
      root.innerHTML = showUnavailable
        ? `${renderUnavailable()}${renderErrorEmpty()}${footer}`
        : `${renderEmpty()}${footer}`;
      return;
    }
    const bannerHtml = showUnavailable ? renderUnavailable() : '';
    root.innerHTML = `${bannerHtml}${renderPanel(top3Entries, selectedIndex)}${footer}`;
  }

  async function refreshAssistantTop3() {
    if (refreshPromise) {
      return refreshPromise;
    }

    refreshPromise = (async () => {
      const hasSnapshot = lastSuccessfulTop3 !== null;
      if (!hasSnapshot) {
        root.innerHTML = '<div class="read-later-assistant-loading">Loading…</div>';
      }

      try {
        const entries = await loadAssistantEntries();
        if (disposed) return;
        top3Entries = selectTop3Latest(entries);
        lastSuccessfulTop3 = top3Entries;
        selectedIndex = 0;
        renderCurrent();
      } catch {
        if (disposed) return;
        if (lastSuccessfulTop3 !== null) {
          top3Entries = lastSuccessfulTop3;
          renderCurrent({ showUnavailable: true });
        } else {
          top3Entries = [];
          selectedIndex = 0;
          root.innerHTML = `${renderErrorEmpty()}${renderManageLink(showManage)}`;
        }
      }
    })().finally(() => {
      refreshPromise = null;
    });

    return refreshPromise;
  }

  const onClick = (event) => {
    if (event.target.closest('.read-later-assistant-manage-link')) {
      if (typeof openReadLater === 'function') openReadLater();
      else if (typeof navigate === 'function') navigate('#/read-later');
      return;
    }

    const pickerItem = event.target.closest('.read-later-assistant-picker-item');
    if (pickerItem) {
      const index = Number(pickerItem.dataset.index);
      if (!Number.isNaN(index) && index >= 0 && index < top3Entries.length) {
        selectedIndex = index;
        renderCurrent();
      }
      return;
    }

    const entryLink = event.target.closest('.read-later-assistant-entry-link');
    if (entryLink) {
      event.preventDefault();
      const url = entryLink.dataset.url || entryLink.getAttribute('href');
      const id = entryLink.dataset.entryId;
      if (!url) return;
      const current = top3Entries[selectedIndex];
      void openExternalUrl(url)
        .then(() => {
          if (disposed || !id || current?.read) return;
          return markEntryRead(id);
        })
        .then(() => {
          if (disposed || !id || current?.read) return;
          top3Entries = top3Entries.map((entry) =>
            entry.id === id ? { ...entry, read: true } : entry,
          );
          lastSuccessfulTop3 = top3Entries;
          renderCurrent();
        })
        .catch((err) => {
          console.error('[read-later-assistant] open link or mark read failed', err);
        });
      return;
    }

    if (event.target.closest('.read-later-assistant-cycle')) {
      if (!top3Entries.length) return;
      selectedIndex = (selectedIndex + 1) % top3Entries.length;
      renderCurrent();
    }
  };

  root.addEventListener('click', onClick);
  const disposeFocusRefresh = bindFocusRefresh(refreshAssistantTop3);
  if (autoLoad) {
    void refreshAssistantTop3();
  }

  function dispose() {
    disposed = true;
    disposeFocusRefresh();
    root.removeEventListener('click', onClick);
    root.innerHTML = '';
  }

  return { dispose, refresh: refreshAssistantTop3 };
}

/** A11y / brand label for the Read Later content region (shell owns overlay title). */
export const READ_LATER_CONTENT_LABEL = 'Open Read Later assistant';

/**
 * Read Later content adapter for the home-entry shell content slot.
 * Shell owns overlay chrome; this module only paints list content into the slot.
 * @returns {{ mount: (slotEl: HTMLElement, ctx?: { host?: { navigate?: Function, openReadLater?: Function } }) => { unmount: () => void } }}
 */
export function createReadLaterContentAdapter() {
  return {
    /**
     * @param {HTMLElement} slotEl
     * @param {{ host?: { navigate?: (hash: string) => void, openReadLater?: () => void } }} [ctx]
     */
    mount(slotEl, ctx = {}) {
      const host = ctx.host ?? {};
      slotEl.setAttribute('aria-label', READ_LATER_CONTENT_LABEL);
      const panel = mountReadLaterAssistant(slotEl, {
        autoLoad: true,
        navigate: typeof host.navigate === 'function' ? host.navigate : undefined,
        openReadLater: typeof host.openReadLater === 'function' ? host.openReadLater : undefined,
      });
      return {
        unmount() {
          panel.dispose();
          slotEl.removeAttribute('aria-label');
        },
      };
    },
  };
}

/**
 * @param {HTMLElement} slotEl
 * @param {{ navigate?: (hash: string) => void, openReadLater?: () => void }} [host]
 * @returns {{ unmount: () => void }}
 */
export function mountReadLaterContent(slotEl, host) {
  return createReadLaterContentAdapter().mount(slotEl, { host });
}
