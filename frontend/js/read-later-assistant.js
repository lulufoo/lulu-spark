import {
  bindFocusRefresh,
  loadReadLaterEntries,
  markEntryRead,
  openExternalUrl,
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

function renderEmpty() {
  return `
    <div class="read-later-assistant-empty read-later-assistant-state">
      <p class="read-later-assistant-state-title">暂无待读</p>
      <p class="read-later-assistant-state-detail">用 Chrome 插件保存后，最新未读会出现在这里</p>
    </div>
  `;
}

function renderUnavailable(message = UNAVAILABLE_MSG) {
  return `<div class="read-later-assistant-unavailable read-later-assistant-banner">${escHtml(message)}</div>`;
}

function renderErrorEmpty(message = UNAVAILABLE_MSG) {
  return `
    <div class="read-later-assistant-empty read-later-assistant-state read-later-assistant-state--error">
      <p class="read-later-assistant-state-title">暂时无法加载</p>
      <p class="read-later-assistant-state-detail">${escHtml(message)}</p>
    </div>
  `;
}

function renderPanel(entries, selectedIndex) {
  const current = entries[selectedIndex];
  const showCycle = entries.length > 1;
  const pickerHtml = showCycle
    ? `<div class="read-later-assistant-picker" role="tablist" aria-label="未读条目">${entries
        .map((entry, index) => {
          const activeClass =
            index === selectedIndex ? ' read-later-assistant-picker-item--active' : '';
          const label = entry.title || entry.url;
          return `<button type="button" class="read-later-assistant-picker-item${activeClass}" data-entry-id="${escHtml(entry.id)}" data-index="${index}" title="${escHtml(label)}"><span class="read-later-assistant-picker-rank">${index + 1}</span><span class="read-later-assistant-picker-label">${escHtml(label)}</span></button>`;
        })
        .join('')}</div>`
    : '';

  const url = escHtml(current.url);
  const title = escHtml(current.title || current.url);
  const cycleHtml = showCycle
    ? '<button type="button" class="read-later-assistant-cycle" aria-label="下一篇">›</button>'
    : '';
  const currentClass = showCycle
    ? 'read-later-assistant-current read-later-assistant-current--has-cycle'
    : 'read-later-assistant-current';

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
  return `<button type="button" class="read-later-assistant-manage-link">查看全部待读 →</button>`;
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
          top3Entries = top3Entries.filter((entry) => entry.id !== id);
          lastSuccessfulTop3 = top3Entries;
          if (selectedIndex >= top3Entries.length) {
            selectedIndex = Math.max(0, top3Entries.length - 1);
          }
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

/**
 * Fixed bottom-right launcher with a non-draggable popover panel (main window only).
 * @param {HTMLElement} [anchor]
 * @param {{ navigate?: (hash: string) => void, openReadLater?: () => void }} [opts]
 */
export function mountReadLaterAssistantWidget(anchor = document.body, opts = {}) {
  const { navigate, openReadLater } = opts;
  const widget = document.createElement('div');
  widget.className = 'rl-assistant-widget';
  widget.innerHTML = `
    <div class="rl-assistant-popover" hidden>
      <header class="rl-assistant-popover-header">
        <span class="rl-assistant-popover-title">待读助手</span>
        <button type="button" class="rl-assistant-close" aria-label="关闭">×</button>
      </header>
      <div class="rl-assistant-popover-body"></div>
    </div>
    <button type="button" class="rl-assistant-fab" aria-label="打开待读助手" aria-expanded="false" title="待读助手">
      <svg class="rl-assistant-fab-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
        <path fill="currentColor" d="M11.2 2.2a.9.9 0 0 1 .6 0l1.1 4.4 4.4 1.1a.9.9 0 0 1 0 1.7l-4.4 1.1-1.1 4.4a.9.9 0 0 1-1.7 0l-1.1-4.4-4.4-1.1a.9.9 0 0 1 0-1.7l4.4-1.1 1.1-4.4z"/>
        <path fill="currentColor" d="M18.2 13.8a.7.7 0 0 1 .5 0l.8 3.2 3.2.8a.7.7 0 0 1 0 1.3l-3.2.8-.8 3.2a.7.7 0 0 1-1.3 0l-.8-3.2-3.2-.8a.7.7 0 0 1 0-1.3l3.2-.8.8-3.2z"/>
        <path fill="currentColor" d="M6.4 15.6a.5.5 0 0 1 .4 0l.5 2.1 2.1.5a.5.5 0 0 1 0 .9l-2.1.5-.5 2.1a.5.5 0 0 1-.9 0l-.5-2.1-2.1-.5a.5.5 0 0 1 0-.9l2.1-.5.5-2.1z"/>
      </svg>
    </button>
  `;
  anchor.appendChild(widget);

  const popover = widget.querySelector('.rl-assistant-popover');
  const body = widget.querySelector('.rl-assistant-popover-body');
  const fab = widget.querySelector('.rl-assistant-fab');
  const closeBtn = widget.querySelector('.rl-assistant-close');

  let panel = null;
  let open = false;

  function setOpen(next) {
    open = next;
    popover.hidden = !open;
    fab.setAttribute('aria-expanded', String(open));
    fab.classList.toggle('rl-assistant-fab--active', open);
    if (!open) return;
    if (!panel) {
      const panelNavigate =
        typeof navigate === 'function'
          ? (hash) => {
              setOpen(false);
              navigate(hash);
            }
          : undefined;
      panel = mountReadLaterAssistant(body, {
        autoLoad: true,
        navigate: panelNavigate,
        openReadLater,
      });
      return;
    }
    void panel.refresh();
  }

  fab.addEventListener('click', (event) => {
    event.stopPropagation();
    setOpen(!open);
  });
  closeBtn.addEventListener('click', (event) => {
    event.stopPropagation();
    setOpen(false);
  });

  const onDocClick = (event) => {
    if (!open) return;
    if (widget.contains(event.target)) return;
    setOpen(false);
  };
  document.addEventListener('click', onDocClick, true);

  function dispose() {
    document.removeEventListener('click', onDocClick, true);
    panel?.dispose();
    widget.remove();
  }

  return { dispose, setOpen };
}

const bootstrapRoot = document.getElementById('read-later-assistant-root');
if (bootstrapRoot) {
  mountReadLaterAssistantWidget(document.body);
}
