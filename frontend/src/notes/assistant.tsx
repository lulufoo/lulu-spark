import { flushSync } from 'react-dom';
import { createRoot, type Root } from 'react-dom/client';
import { fetchIndex } from '../host/api.ts';
import { normalizeCorpusIndex } from '../corpus/corpus-index.ts';
import { filenameFromPath, slugToTitle } from '../shared/utils.ts';

const UNAVAILABLE_MSG = 'List temporarily unavailable — try again later';

type AssistantNote = {
  title?: string;
  common_path?: string;
  source_type?: string;
  created_at?: string;
};

export function selectTopNotesByCreatedAt(entries: AssistantNote[]) {
  return [...entries]
    .filter((entry) => entry?.source_type === 'note')
    .sort((a, b) => String(b.created_at ?? '').localeCompare(String(a.created_at ?? '')))
    .slice(0, 3);
}

export async function loadAssistantNotes() {
  const data = await fetchIndex();
  const map = normalizeCorpusIndex(data);
  return Object.values(map) as AssistantNote[];
}

function noteLabel(entry: AssistantNote) {
  if (entry.title) return String(entry.title);
  if (entry.common_path) {
    return slugToTitle(filenameFromPath(entry.common_path));
  }
  return 'Untitled note';
}

function EmptyState() {
  return (
    <div className="note-assistant-empty note-assistant-state">
      <p className="note-assistant-state-title">No notes yet</p>
      <p className="note-assistant-state-detail">Create below; latest notes appear here</p>
    </div>
  );
}

function ErrorEmpty({ message = UNAVAILABLE_MSG }: { message?: string }) {
  return (
    <div className="note-assistant-empty note-assistant-state note-assistant-state--error">
      <p className="note-assistant-state-title">Unable to load</p>
      <p className="note-assistant-state-detail">{message}</p>
    </div>
  );
}

function NoteList({ notes }: { notes: AssistantNote[] }) {
  return (
    <ul className="note-assistant-list">
      {notes.map((entry) => {
        const path = entry.common_path ?? '';
        const label = noteLabel(entry);
        return (
          <li key={path || label} className="note-assistant-item" data-common-path={path}>
            <button type="button" className="note-assistant-item-link" data-common-path={path}>
              <span className="note-assistant-item-summary">{label}</span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}

function CreateButton() {
  return (
    <button type="button" className="note-assistant-create">
      New note
    </button>
  );
}

function bindFocusRefresh(refresh: () => unknown) {
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

export function mountNoteAssistant(
  root: HTMLElement,
  opts: { autoLoad?: boolean; openCreateNote?: (opts?: object) => void | Promise<void> } = {},
) {
  const { autoLoad = true, openCreateNote } = opts;
  let disposed = false;
  let topNotes: AssistantNote[] = [];
  let refreshPromise: Promise<void> | null = null;
  let reactRoot: Root | null = createRoot(root);
  let errorMessage: string | null = null;

  const showCreate = typeof openCreateNote === 'function';

  function paint(loading = false) {
    if (!reactRoot || disposed) return;
    flushSync(() => {
      if (loading) {
        reactRoot!.render(<div className="note-assistant-loading">Loading…</div>);
        return;
      }
      if (errorMessage) {
        reactRoot!.render(
          <>
            <ErrorEmpty message={errorMessage} />
            {showCreate ? <CreateButton /> : null}
          </>,
        );
        return;
      }
      if (!topNotes.length) {
        reactRoot!.render(
          <>
            <EmptyState />
            {showCreate ? <CreateButton /> : null}
          </>,
        );
        return;
      }
      reactRoot!.render(
        <>
          <NoteList notes={topNotes} />
          {showCreate ? <CreateButton /> : null}
        </>,
      );
    });
  }

  async function refreshAssistantTopNotes() {
    if (refreshPromise) {
      return refreshPromise;
    }

    refreshPromise = (async () => {
      if (!topNotes.length) {
        paint(true);
      }

      try {
        const entries = await loadAssistantNotes();
        if (disposed) return;
        topNotes = selectTopNotesByCreatedAt(entries);
        errorMessage = null;
        paint();
      } catch {
        if (disposed) return;
        topNotes = [];
        errorMessage = UNAVAILABLE_MSG;
        paint();
      }
    })().finally(() => {
      refreshPromise = null;
    });

    return refreshPromise;
  }

  const onClick = (event: Event) => {
    const target = event.target as HTMLElement | null;
    if (!target) return;
    const createBtn = target.closest('.note-assistant-create');
    if (createBtn) {
      if (typeof openCreateNote === 'function') {
        void openCreateNote();
      }
      return;
    }

    const itemLink = target.closest('.note-assistant-item-link') as HTMLElement | null;
    if (itemLink) {
      const commonPath = itemLink.dataset.commonPath;
      if (!commonPath) return;
      document.dispatchEvent(new CustomEvent('cta:open-entry', { detail: { common_path: commonPath } }));
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
    flushSync(() => {
      reactRoot?.unmount();
    });
    reactRoot = null;
    root.innerHTML = '';
  }

  return { dispose, refresh: refreshAssistantTopNotes };
}

/** Brand label retained for copy-lock / a11y (overlay title comes from EntryConfig). */
export const NOTES_CONTENT_LABEL = 'Notes Assistant';

export function createNotesContentAdapter() {
  return {
    mount(
      slotEl: HTMLElement,
      ctx: { host?: { openCreateNote?: (opts?: object) => void | Promise<void> } } = {},
    ) {
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
