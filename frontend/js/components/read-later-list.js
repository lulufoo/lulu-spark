import { createApiClient, resolveReadDriver } from '../apiClient.js';
import { escHtml } from '../utils.js';

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

function renderList(container, entries) {
  if (!entries.length) {
    container.innerHTML = '<div class="read-later-empty">暂无待读</div>';
    return;
  }
  container.innerHTML = `<ul class="read-later-list">${entries.map(renderEntry).join('')}</ul>`;
}

function updateEntryRead(container, id) {
  const item = container.querySelector(`[data-entry-id="${id}"]`);
  if (!item) return;
  item.classList.remove('read-later-item--unread');
  item.classList.add('read-later-item--read');
  item.querySelector('.read-later-mark-read')?.remove();
}

export function mountReadLaterList(container) {
  let disposed = false;

  container.innerHTML = '<div class="read-later-loading">加载中…</div>';

  async function refresh() {
    try {
      const entries = await loadReadLaterEntries();
      if (disposed) return;
      renderList(container, entries);
    } catch (err) {
      if (disposed) return;
      if (err?.status === 503) {
        container.innerHTML =
          '<div class="read-later-unavailable">请先启动 Workbench</div>';
        return;
      }
      container.innerHTML = `<div class="read-later-error">${escHtml(err?.message || '加载失败')}</div>`;
    }
  }

  const onClick = (event) => {
    const btn = event.target.closest('.read-later-mark-read');
    if (!btn) return;
    const { id } = btn.dataset;
    if (!id) return;
    btn.disabled = true;
    void markEntryRead(id)
      .then(() => {
        if (disposed) return;
        updateEntryRead(container, id);
      })
      .catch((err) => {
        if (disposed) return;
        btn.disabled = false;
        console.error('markEntryRead failed', err);
      });
  };

  container.addEventListener('click', onClick);
  void refresh();

  return {
    unmount() {
      disposed = true;
      container.removeEventListener('click', onClick);
      container.innerHTML = '';
    },
  };
}
