import { fetchIndex } from '../host/api.js';
import { normalizeCorpusIndex } from '../corpus/corpus-index.js';
import { escHtml, filenameFromPath, slugToTitle } from '../shared/utils.js';

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

/** Brand label retained for copy-lock / a11y (overlay title comes from EntryConfig). */
export const NOTES_CONTENT_LABEL = 'Notes Assistant';

/**
 * Notes content adapter for the home-entry shell content slot.
 * Shell owns overlay chrome; this module only paints note content into the slot.
 * @returns {{ mount: (slotEl: HTMLElement, ctx?: { host?: { openCreateNote?: Function } }) => { unmount: () => void } }}
 */
export function createNotesContentAdapter() {
  return {
    /**
     * @param {HTMLElement} slotEl
     * @param {{ host?: { openCreateNote?: (opts?: object) => void|Promise<void> } }} [ctx]
     */
    mount(slotEl, ctx = {}) {
      const host = ctx.host ?? {};
      slotEl.setAttribute('aria-label', NOTES_CONTENT_LABEL);
      const panel = mountNoteAssistant(slotEl, {
        autoLoad: true,
        openCreateNote: typeof host.openCreateNote === 'function' ? host.openCreateNote : undefined,
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
 * @param {{ openCreateNote?: (opts?: object) => void|Promise<void> }} [host]
 * @returns {{ unmount: () => void }}
 */
export function mountNotesContent(slotEl, host) {
  return createNotesContentAdapter().mount(slotEl, { host });
}
