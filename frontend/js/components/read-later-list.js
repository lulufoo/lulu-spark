import { createApiClient, resolveReadDriver } from '../apiClient.js';
import { escHtml } from '../utils.js';

const UNAVAILABLE_MSG = '列表暂时不可用，请稍后重试';

function getTauriInvoke() {
  if (typeof window === 'undefined') return null;
  const invoke =
    window.__TAURI__?.core?.invoke || window.__TAURI_INTERNALS__?.invoke;
  return typeof invoke === 'function' ? invoke : null;
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

export function bindFocusRefresh(refresh) {
  const onFocus = () => {
    void refresh();
  };
  const onVisibility = () => {
    if (document.visibilityState === 'visible') {
      void refresh();
    }
  };
  window.addEventListener('focus', onFocus);
  document.addEventListener('visibilitychange', onVisibility);
  return () => {
    window.removeEventListener('focus', onFocus);
    document.removeEventListener('visibilitychange', onVisibility);
  };
}

function renderEntry(entry) {
  const readClass = entry.read
    ? 'read-later-item--read'
    : 'read-later-item--unread';
  const markBtn = entry.read
    ? ''
    : `<button type="button" class="read-later-mark-read" data-id="${escHtml(entry.id)}">标记已读</button>`;
  return `
    <li class="read-later-item ${readClass}" data-entry-id="${escHtml(entry.id)}">
      <div class="read-later-title">${escHtml(entry.title || entry.url)}</div>
      <div class="read-later-url">${escHtml(entry.url)}</div>
      <div class="read-later-saved-at">${escHtml(entry.saved_at ?? '')}</div>
      ${markBtn}
    </li>
  `;
}

function renderUnavailableBanner(message = UNAVAILABLE_MSG) {
  return `<div class="read-later-unavailable">${escHtml(message)}</div>`;
}

function renderErrorEmpty(message = UNAVAILABLE_MSG) {
  return `
    <div class="read-later-empty">暂无待读</div>
    ${renderUnavailableBanner(message)}
  `;
}

function renderList(container, entries, { showUnavailable = false } = {}) {
  const bannerHtml = showUnavailable ? renderUnavailableBanner() : '';
  if (!entries.length) {
    container.innerHTML = showUnavailable
      ? renderErrorEmpty()
      : '<div class="read-later-empty">暂无待读</div>';
    return;
  }
  container.innerHTML = `${bannerHtml}<ul class="read-later-list">${entries.map(renderEntry).join('')}</ul>`;
}

function updateEntryRead(container, id) {
  const item = container.querySelector(`[data-entry-id="${id}"]`);
  if (!item) return;
  item.classList.remove('read-later-item--unread');
  item.classList.add('read-later-item--read');
  item.querySelector('.read-later-mark-read')?.remove();
  item.querySelector('.read-later-action-error')?.remove();
}

export function mountReadLaterList(container) {
  let disposed = false;
  let lastSuccessfulEntries = null;
  let refreshPromise = null;

  container.innerHTML = '<div class="read-later-loading">加载中…</div>';

  async function refresh() {
    if (refreshPromise) {
      return refreshPromise;
    }

    refreshPromise = (async () => {
      const hasSnapshot = lastSuccessfulEntries !== null;
      if (!hasSnapshot) {
        container.innerHTML = '<div class="read-later-loading">加载中…</div>';
      }

      try {
        const entries = await loadReadLaterEntries();
        if (disposed) return;
        lastSuccessfulEntries = entries;
        renderList(container, entries);
      } catch (err) {
        if (disposed) return;
        if (lastSuccessfulEntries !== null) {
          renderList(container, lastSuccessfulEntries, { showUnavailable: true });
        } else {
          container.innerHTML = renderErrorEmpty();
        }
      }
    })().finally(() => {
      refreshPromise = null;
    });

    return refreshPromise;
  }

  const disposeFocusRefresh = bindFocusRefresh(refresh);

  const onClick = (event) => {
    const btn = event.target.closest('.read-later-mark-read');
    if (!btn) return;
    const { id } = btn.dataset;
    if (!id) return;
    btn.disabled = true;
    btn.closest('.read-later-item')?.querySelector('.read-later-action-error')?.remove();
    void markEntryRead(id)
      .then(() => {
        if (disposed) return;
        updateEntryRead(container, id);
        if (lastSuccessfulEntries) {
          lastSuccessfulEntries = lastSuccessfulEntries.map((entry) =>
            entry.id === id ? { ...entry, read: true } : entry,
          );
        }
      })
      .catch((err) => {
        if (disposed) return;
        btn.disabled = false;
        const item = btn.closest('.read-later-item');
        if (!item) return;
        const errEl = document.createElement('div');
        errEl.className = 'read-later-action-error';
        errEl.textContent = err?.message || '标记已读失败';
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

  return { dispose, unmount: dispose };
}
