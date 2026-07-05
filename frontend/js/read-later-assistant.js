import {
  bindFocusRefresh,
  loadReadLaterEntries,
} from './components/read-later-list.js';
import { escHtml } from './utils.js';

export { bindFocusRefresh };

const UNAVAILABLE_MSG = '列表暂时不可用，请稍后重试';

export async function loadAssistantEntries() {
  return loadReadLaterEntries();
}

export function selectTop3Unread(entries) {
  return entries
    .filter((entry) => !entry.read)
    .sort((a, b) => {
      const aTime = Date.parse(a.saved_at ?? '') || 0;
      const bTime = Date.parse(b.saved_at ?? '') || 0;
      return bTime - aTime;
    })
    .slice(0, 3);
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

function renderEmpty(message = '暂无待读') {
  return `<div class="read-later-assistant-empty">${escHtml(message)}</div>`;
}

function renderUnavailable(message = UNAVAILABLE_MSG) {
  return `<div class="read-later-assistant-unavailable">${escHtml(message)}</div>`;
}

function renderErrorEmpty(message = UNAVAILABLE_MSG) {
  return `${renderEmpty()}${renderUnavailable(message)}`;
}

function renderPanel(entries, selectedIndex) {
  const current = entries[selectedIndex];
  const pickerItems = entries
    .map((entry, index) => {
      const activeClass =
        index === selectedIndex ? ' read-later-assistant-picker-item--active' : '';
      return `<button type="button" class="read-later-assistant-picker-item${activeClass}" data-entry-id="${escHtml(entry.id)}" data-index="${index}">${escHtml(entry.title || entry.url)}</button>`;
    })
    .join('');

  return `
    <div class="read-later-assistant-panel">
      <div class="read-later-assistant-picker">${pickerItems}</div>
      <div class="read-later-assistant-current">
        <div class="read-later-assistant-current-title">${escHtml(current.title || current.url)}</div>
        <div class="read-later-assistant-current-url">${escHtml(current.url)}</div>
        <div class="read-later-assistant-current-saved-at">${escHtml(current.saved_at ?? '')}</div>
        <div class="read-later-assistant-nav">
          <button type="button" class="read-later-assistant-prev">上一篇</button>
          <button type="button" class="read-later-assistant-open-link">打开链接</button>
          <button type="button" class="read-later-assistant-next">下一篇</button>
        </div>
      </div>
    </div>
  `;
}

export function mountReadLaterAssistant(root) {
  let disposed = false;
  let top3Entries = [];
  let selectedIndex = 0;
  let lastSuccessfulTop3 = null;
  let refreshPromise = null;

  function renderCurrent({ showUnavailable = false } = {}) {
    if (!top3Entries.length) {
      root.innerHTML = showUnavailable ? renderErrorEmpty() : renderEmpty();
      return;
    }
    const bannerHtml = showUnavailable ? renderUnavailable() : '';
    root.innerHTML = `${bannerHtml}${renderPanel(top3Entries, selectedIndex)}`;
  }

  async function refreshAssistantTop3() {
    if (refreshPromise) {
      return refreshPromise;
    }

    refreshPromise = (async () => {
      const hasSnapshot = lastSuccessfulTop3 !== null;
      if (!hasSnapshot) {
        root.innerHTML = '<div class="read-later-assistant-loading">加载中…</div>';
      }

      try {
        const entries = await loadAssistantEntries();
        if (disposed) return;
        top3Entries = selectTop3Unread(entries);
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
          root.innerHTML = renderErrorEmpty();
        }
      }
    })().finally(() => {
      refreshPromise = null;
    });

    return refreshPromise;
  }

  const onClick = (event) => {
    const pickerItem = event.target.closest('.read-later-assistant-picker-item');
    if (pickerItem) {
      const index = Number(pickerItem.dataset.index);
      if (!Number.isNaN(index) && index >= 0 && index < top3Entries.length) {
        selectedIndex = index;
        renderCurrent();
      }
      return;
    }

    if (event.target.closest('.read-later-assistant-prev')) {
      if (!top3Entries.length) return;
      selectedIndex =
        (selectedIndex - 1 + top3Entries.length) % top3Entries.length;
      renderCurrent();
      return;
    }

    if (event.target.closest('.read-later-assistant-next')) {
      if (!top3Entries.length) return;
      selectedIndex = (selectedIndex + 1) % top3Entries.length;
      renderCurrent();
      return;
    }

    if (event.target.closest('.read-later-assistant-open-link')) {
      const current = top3Entries[selectedIndex];
      if (!current?.url) return;
      void openExternalUrl(current.url).catch((err) => {
        console.error('[read-later-assistant] open link failed', err);
      });
    }
  };

  root.addEventListener('click', onClick);
  const disposeFocusRefresh = bindFocusRefresh(refreshAssistantTop3);
  void refreshAssistantTop3();

  function dispose() {
    disposed = true;
    disposeFocusRefresh();
    root.removeEventListener('click', onClick);
    root.innerHTML = '';
  }

  return { dispose };
}

const bootstrapRoot = document.getElementById('read-later-assistant-root');
if (bootstrapRoot) {
  mountReadLaterAssistant(bootstrapRoot);
}
