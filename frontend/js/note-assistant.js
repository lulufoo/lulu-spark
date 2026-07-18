import { fetchIndex } from './api.js';
import { normalizeCorpusIndex } from './corpus-index.js';
import { escHtml, filenameFromPath, slugToTitle } from './utils.js';

const UNAVAILABLE_MSG = 'List temporarily unavailable — try again later';

/**
 * Filter note entries, sort by created_at desc, take ≤3.
 * Sort key is created_at only (corpus compact timestamps; not updated_at).
 * @param {object[]} entries
 * @returns {object[]}
 */
export function selectTopNotesByCreatedAt(entries) {
  return [...entries]
    .filter((entry) => entry?.source_type === 'note')
    .sort((a, b) =>
      String(b.created_at ?? '').localeCompare(String(a.created_at ?? '')),
    )
    .slice(0, 3);
}

export async function loadAssistantNotes() {
  const data = await fetchIndex();
  const map = normalizeCorpusIndex(data);
  return Object.values(map);
}

function noteLabel(entry) {
  if (entry.title) return String(entry.title);
  if (entry.common_path) {
    return slugToTitle(filenameFromPath(entry.common_path));
  }
  return 'Untitled note';
}

function renderEmpty() {
  return `
    <div class="note-assistant-empty note-assistant-state">
      <p class="note-assistant-state-title">No notes yet</p>
      <p class="note-assistant-state-detail">Create below; latest notes appear here</p>
    </div>
  `;
}

function renderErrorEmpty(message = UNAVAILABLE_MSG) {
  return `
    <div class="note-assistant-empty note-assistant-state note-assistant-state--error">
      <p class="note-assistant-state-title">Unable to load</p>
      <p class="note-assistant-state-detail">${escHtml(message)}</p>
    </div>
  `;
}

function renderNoteList(notes) {
  const items = notes
    .map((entry) => {
      const path = entry.common_path ?? '';
      const label = noteLabel(entry);
      return `
        <li class="note-assistant-item" data-common-path="${escHtml(path)}">
          <button type="button" class="note-assistant-item-link" data-common-path="${escHtml(path)}">
            <span class="note-assistant-item-summary">${escHtml(label)}</span>
          </button>
        </li>
      `;
    })
    .join('');
  return `<ul class="note-assistant-list">${items}</ul>`;
}

function renderCreateButton(showCreate) {
  if (!showCreate) return '';
  return `<button type="button" class="note-assistant-create">New note</button>`;
}

function bindFocusRefresh(refresh) {
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

/**
 * @param {HTMLElement} root
 * @param {{ autoLoad?: boolean, openCreateNote?: (opts?: object) => void|Promise<void> }} [opts]
 */
export function mountNoteAssistant(root, opts = {}) {
  const { autoLoad = true, openCreateNote } = opts;
  let disposed = false;
  let topNotes = [];
  let refreshPromise = null;

  const showCreate = typeof openCreateNote === 'function';

  function renderCurrent() {
    const footer = renderCreateButton(showCreate);
    if (!topNotes.length) {
      root.innerHTML = `${renderEmpty()}${footer}`;
      return;
    }
    root.innerHTML = `${renderNoteList(topNotes)}${footer}`;
  }

  async function refreshAssistantTopNotes() {
    if (refreshPromise) {
      return refreshPromise;
    }

    refreshPromise = (async () => {
      if (!topNotes.length) {
        root.innerHTML = '<div class="note-assistant-loading">Loading…</div>';
      }

      try {
        const entries = await loadAssistantNotes();
        if (disposed) return;
        topNotes = selectTopNotesByCreatedAt(entries);
        renderCurrent();
      } catch {
        if (disposed) return;
        topNotes = [];
        root.innerHTML = `${renderErrorEmpty()}${renderCreateButton(showCreate)}`;
      }
    })().finally(() => {
      refreshPromise = null;
    });

    return refreshPromise;
  }

  const onClick = (event) => {
    const createBtn = event.target.closest('.note-assistant-create');
    if (createBtn) {
      if (typeof openCreateNote === 'function') {
        void openCreateNote();
      }
      return;
    }

    const itemLink = event.target.closest('.note-assistant-item-link');
    if (itemLink) {
      const commonPath = itemLink.dataset.commonPath;
      if (!commonPath) return;
      document.dispatchEvent(
        new CustomEvent('cta:open-entry', { detail: { common_path: commonPath } }),
      );
    }
  };

  root.addEventListener('click', onClick);
  const disposeFocusRefresh = bindFocusRefresh(refreshAssistantTopNotes);
  if (autoLoad) {
    void refreshAssistantTopNotes();
  }

  function dispose() {
    disposed = true;
    disposeFocusRefresh();
    root.removeEventListener('click', onClick);
    root.innerHTML = '';
  }

  return { dispose, refresh: refreshAssistantTopNotes };
}

/**
 * Fixed bottom-right note FAB launcher (stack offset / mutual exclusion: t6).
 * @param {HTMLElement} [anchor]
 * @param {{ openCreateNote?: (opts?: object) => void|Promise<void> }} [opts]
 */
export function mountNoteAssistantWidget(anchor = document.body, opts = {}) {
  const { openCreateNote } = opts;
  const widget = document.createElement('div');
  widget.className = 'note-assistant-widget';
  widget.innerHTML = `
    <div class="note-assistant-popover" hidden>
      <header class="note-assistant-popover-header">
        <span class="note-assistant-popover-title">Notes Assistant</span>
        <button type="button" class="note-assistant-close" aria-label="Close">×</button>
      </header>
      <div class="note-assistant-popover-body"></div>
    </div>
    <button type="button" class="note-assistant-fab" aria-label="Open Notes Assistant" aria-expanded="false" title="Notes Assistant">
      <svg class="note-assistant-fab-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
        <path fill="currentColor" d="M6 3.5A1.5 1.5 0 0 0 4.5 5v14A1.5 1.5 0 0 0 6 20.5h9.5a.75.75 0 0 0 .53-.22l3.25-3.25a.75.75 0 0 0 .22-.53V5A1.5 1.5 0 0 0 18 3.5H6zm8.75 13.25V19H6.5V5.5h11v9.75H15.5a.75.75 0 0 0-.75.75z"/>
      </svg>
    </button>
  `;
  anchor.appendChild(widget);

  const popover = widget.querySelector('.note-assistant-popover');
  const body = widget.querySelector('.note-assistant-popover-body');
  const fab = widget.querySelector('.note-assistant-fab');
  const closeBtn = widget.querySelector('.note-assistant-close');

  let panel = null;
  let open = false;

  function setOpen(next) {
    open = next;
    popover.hidden = !open;
    fab.setAttribute('aria-expanded', String(open));
    fab.classList.toggle('note-assistant-fab--active', open);
    if (!open) return;
    if (!panel) {
      const panelOpenCreate =
        typeof openCreateNote === 'function'
          ? (...args) => {
              setOpen(false);
              return openCreateNote(...args);
            }
          : undefined;
      panel = mountNoteAssistant(body, {
        autoLoad: true,
        openCreateNote: panelOpenCreate,
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
