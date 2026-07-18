import { createApiClient, resolveReadDriver } from '../apiClient.js';
import { escHtml } from '../utils.js';

const UNAVAILABLE_MSG = 'List temporarily unavailable. Please try again later';

function getTauriInvoke() {
  if (typeof window === 'undefined') return null;
  const invoke =
    window.__TAURI__?.core?.invoke || window.__TAURI_INTERNALS__?.invoke;
  return typeof invoke === 'function' ? invoke : null;
}

function getTauriOpener() {
  if (typeof window === 'undefined') return null;
  const openUrl = window.__TAURI__?.opener?.openUrl;
  return typeof openUrl === 'function' ? openUrl.bind(window.__TAURI__.opener) : null;
}

export async function openExternalUrl(url) {
  const openUrl = getTauriOpener();
  if (!openUrl) {
    throw new Error('Tauri opener unavailable');
  }
  await openUrl(url);
}

function serviceError(data) {
  if (!data || typeof data !== 'object' || Array.isArray(data)) return null;
  if (!data.error) return null;
  const err = new Error(String(data.error));
  err.status = typeof data._status === 'number' ? data._status : 500;
  return err;
}

function httpStatusError(error) {
  const message = error instanceof Error ? error.message : String(error);
  const match = message.match(/^HTTP (\d+)/);
  if (!match) return null;
  const err = new Error(message);
  err.status = Number(match[1]);
  return err;
}

function formatSavedAt(savedAt) {
  if (!savedAt) return '';
  const date = new Date(savedAt);
  if (Number.isNaN(date.getTime())) return savedAt;
  return date.toLocaleString('zh-CN', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export async function loadReadLaterEntries() {
  const mode = resolveReadDriver();
  const client = createApiClient(resolveReadDriver(mode));
  try {
    const data = await client.getJson('/api/read-later');
    const unavailable = serviceError(data);
    if (unavailable) throw unavailable;
    return Array.isArray(data) ? data : [];
  } catch (error) {
    const httpErr = httpStatusError(error);
    if (httpErr) throw httpErr;
    throw error;
  }
}

export async function markEntryRead(id) {
  const invoke = getTauriInvoke();
  if (!invoke) {
    throw new Error('Tauri invoke unavailable');
  }
  const result = await invoke('mark_read_later', { id, read: true });
  const unavailable = serviceError(result);
  if (unavailable) throw unavailable;
}

export async function deleteReadLaterEntry(id) {
  const invoke = getTauriInvoke();
  if (!invoke) {
    throw new Error('Tauri invoke unavailable');
  }
  const result = await invoke('delete_read_later', { id });
  const unavailable = serviceError(result);
  if (unavailable) throw unavailable;
}

export function bindFocusRefresh(refresh) {
  const runRefresh = () => {
    void refresh();
  };
  const onVisibility = () => {
    if (document.visibilityState === 'visible') {
      runRefresh();
    }
  };
  window.addEventListener('focus', runRefresh);
  document.addEventListener('visibilitychange', onVisibility);
  return () => {
    window.removeEventListener('focus', runRefresh);
    document.removeEventListener('visibilitychange', onVisibility);
  };
}

/** @typedef {'all' | 'unread'} ReadLaterFilter */

function sortEntries(entries) {
  return [...entries].sort((a, b) => {
    const aTime = Date.parse(a.saved_at ?? '') || 0;
    const bTime = Date.parse(b.saved_at ?? '') || 0;
    return bTime - aTime;
  });
}

function filterEntries(entries, filter) {
  if (filter === 'unread') {
    return sortEntries(entries.filter((entry) => !entry.read));
  }
  return sortEntries(entries);
}

function renderTabs(activeFilter) {
  const allActive = activeFilter === 'all' ? ' read-later-tab--active' : '';
  const unreadActive = activeFilter === 'unread' ? ' read-later-tab--active' : '';
  return `
    <div class="read-later-tabs" role="tablist" aria-label="Filter read-later">
      <button type="button" class="read-later-tab${allActive}" data-filter="all" role="tab" aria-selected="${activeFilter === 'all'}">All</button>
      <button type="button" class="read-later-tab${unreadActive}" data-filter="unread" role="tab" aria-selected="${activeFilter === 'unread'}">Unread</button>
    </div>
  `;
}

function renderLinkEntry(entry) {
  const title = escHtml(entry.title || entry.url);
  const url = escHtml(entry.url);
  const savedAt = formatSavedAt(entry.saved_at);
  const meta = savedAt ? `<span class="read-later-link-meta">${escHtml(savedAt)}</span>` : '';
  const readClass = entry.read ? ' read-later-item--read' : ' read-later-item--unread';
  return `
    <li class="read-later-item${readClass}" data-entry-id="${escHtml(entry.id)}">
      <a class="read-later-link" href="${url}" data-url="${url}">
        <span class="read-later-link-title">${title}</span>
        ${meta}
      </a>
      <button type="button" class="read-later-delete" data-id="${escHtml(entry.id)}" aria-label="Delete">×</button>
    </li>
  `;
}

function emptyMessage(filter) {
  return filter === 'unread' ? 'No items to read later' : 'No items';
}

function renderUnavailableBanner(message = UNAVAILABLE_MSG) {
  return `<div class="read-later-unavailable">${escHtml(message)}</div>`;
}

function renderErrorEmpty(message = UNAVAILABLE_MSG, filter = 'unread') {
  return `
    <div class="read-later-empty">${emptyMessage(filter)}</div>
    ${renderUnavailableBanner(message)}
  `;
}

function renderListBody(entries, { showUnavailable = false, message = UNAVAILABLE_MSG, filter = 'unread' } = {}) {
  const bannerHtml = showUnavailable ? renderUnavailableBanner(message) : '';
  const displayEntries = filterEntries(entries, filter);
  if (!displayEntries.length) {
    return showUnavailable
      ? renderErrorEmpty(message, filter)
      : `<div class="read-later-empty">${emptyMessage(filter)}</div>`;
  }
  const items = displayEntries.map((entry) => renderLinkEntry(entry)).join('');
  return `${bannerHtml}<ul class="read-later-list">${items}</ul>`;
}

export function renderUnavailableState(
  container,
  { mode, message = UNAVAILABLE_MSG, entries = [], filter = 'unread', showTabs = false } = {},
) {
  if (mode === 'empty') {
    container.innerHTML = showTabs
      ? `${renderTabs(filter)}<div class="read-later-list-host">${renderErrorEmpty(message, filter)}</div>`
      : renderErrorEmpty(message, filter);
    return;
  }
  if (mode === 'retained') {
    container.innerHTML = showTabs
      ? `${renderTabs(filter)}<div class="read-later-list-host">${renderListBody(entries, { showUnavailable: true, message, filter })}</div>`
      : renderListBody(entries, { showUnavailable: true, message, filter });
  }
}

function paintList(host, entries, { filter, showUnavailable = false, message = UNAVAILABLE_MSG } = {}) {
  host.innerHTML = renderListBody(entries, { showUnavailable, message, filter });
}

function updateTabState(container, filter) {
  container.querySelectorAll('.read-later-tab').forEach((tab) => {
    const active = tab.dataset.filter === filter;
    tab.classList.toggle('read-later-tab--active', active);
    tab.setAttribute('aria-selected', String(active));
  });
}

/**
 * @param {HTMLElement} container
 * @param {{ showTabs?: boolean, initialFilter?: ReadLaterFilter }} [opts]
 */
export function mountReadLaterList(container, opts = {}) {
  const { showTabs = false, initialFilter = 'unread' } = opts;
  let filter = initialFilter;
  let disposed = false;
  let lastSuccessfulEntries = null;
  let refreshPromise = null;
  let listHost = container;

  container.innerHTML = '<div class="read-later-loading">Loading…</div>';

  function ensureChrome() {
    if (!showTabs) {
      listHost = container;
      return;
    }
    if (container.querySelector('.read-later-list-host')) {
      listHost = container.querySelector('.read-later-list-host');
      updateTabState(container, filter);
      return;
    }
    container.innerHTML = `${renderTabs(filter)}<div class="read-later-list-host"><div class="read-later-loading">Loading…</div></div>`;
    listHost = container.querySelector('.read-later-list-host');
  }

  function renderEntries({ showUnavailable = false, message = UNAVAILABLE_MSG } = {}) {
    ensureChrome();
    if (!lastSuccessfulEntries) {
      listHost.innerHTML = '<div class="read-later-loading">Loading…</div>';
      return;
    }
    paintList(listHost, lastSuccessfulEntries, { filter, showUnavailable, message });
  }

  async function refresh() {
    if (refreshPromise) {
      return refreshPromise;
    }

    refreshPromise = (async () => {
      const hasSnapshot = lastSuccessfulEntries !== null;
      if (!hasSnapshot) {
        container.innerHTML = '<div class="read-later-loading">Loading…</div>';
      }

      try {
        const entries = await loadReadLaterEntries();
        if (disposed) return;
        lastSuccessfulEntries = entries;
        renderEntries();
      } catch {
        if (disposed) return;
        if (lastSuccessfulEntries !== null) {
          renderUnavailableState(container, {
            mode: 'retained',
            entries: lastSuccessfulEntries,
            filter,
            showTabs,
          });
          listHost = container.querySelector('.read-later-list-host') ?? container;
        } else {
          renderUnavailableState(container, { mode: 'empty', filter, showTabs });
          listHost = container.querySelector('.read-later-list-host') ?? container;
        }
      }
    })().finally(() => {
      refreshPromise = null;
    });

    return refreshPromise;
  }

  const disposeFocusRefresh = bindFocusRefresh(refresh);

  const onClick = (event) => {
    const tab = event.target.closest('.read-later-tab');
    if (tab?.dataset.filter) {
      filter = tab.dataset.filter === 'all' ? 'all' : 'unread';
      updateTabState(container, filter);
      renderEntries();
      return;
    }

    const deleteBtn = event.target.closest('.read-later-delete');
    if (deleteBtn) {
      event.preventDefault();
      const { id } = deleteBtn.dataset;
      if (!id) return;
      const item = deleteBtn.closest('.read-later-item');
      deleteBtn.disabled = true;
      item?.querySelector('.read-later-action-error')?.remove();
      void deleteReadLaterEntry(id)
        .then(() => {
          if (disposed) return;
          if (lastSuccessfulEntries) {
            lastSuccessfulEntries = lastSuccessfulEntries.filter((entry) => entry.id !== id);
          }
          renderEntries();
        })
        .catch((err) => {
          if (disposed || !item) return;
          deleteBtn.disabled = false;
          const errEl = document.createElement('div');
          errEl.className = 'read-later-action-error';
          errEl.textContent = err?.message || 'Delete failed';
          item.appendChild(errEl);
        });
      return;
    }

    const link = event.target.closest('.read-later-link');
    if (!link) return;
    event.preventDefault();
    const item = link.closest('.read-later-item');
    const { entryId: id } = item?.dataset ?? {};
    const url = link.dataset.url || link.getAttribute('href');
    if (!url) return;

    item?.querySelector('.read-later-action-error')?.remove();
    void openExternalUrl(url)
      .then(() => {
        if (disposed || !id) return;
        const entry = lastSuccessfulEntries?.find((e) => e.id === id);
        if (entry?.read) return;
        void markEntryRead(id)
          .then(() => {
            if (disposed) return;
            if (lastSuccessfulEntries) {
              lastSuccessfulEntries = lastSuccessfulEntries.map((e) =>
                e.id === id ? { ...e, read: true } : e,
              );
            }
            renderEntries();
          })
          .catch((err) => {
            if (disposed || !item) return;
            const errEl = document.createElement('div');
            errEl.className = 'read-later-action-error';
            errEl.textContent = err?.message || 'Failed to mark as read';
            item.appendChild(errEl);
          });
      })
      .catch((err) => {
        if (disposed || !item) return;
        const errEl = document.createElement('div');
        errEl.className = 'read-later-action-error';
        errEl.textContent = err?.message || 'Failed to open link';
        item.appendChild(errEl);
      });
  };

  container.addEventListener('click', onClick);
  void refresh();

  function dispose() {
    disposed = true;
    disposeFocusRefresh();
    container.removeEventListener('click', onClick);
    container.innerHTML = '';
  }

  return { dispose, unmount: dispose, refresh };
}
